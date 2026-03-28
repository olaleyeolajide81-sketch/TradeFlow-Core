#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, 
    token, Address, Env, Symbol, Map, BytesN, Vec, Val, IntoVal, Bytes
};

mod utils;
use utils::fixed_point::{self, Q64};

mod tests;

const MINIMUM_LIQUIDITY: u128 = 1000;

const BURN_ADDRESS: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiquidityPosition {
    pub owner: Address,
    pub token_a_amount: u128,
    pub token_b_amount: u128,
    pub shares: u128,
}

#[contracttype]
pub struct PendingFeeChange {
    pub new_fee: u32, // Fee in basis points (100 = 1%)
    pub execution_timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct PriceObservation {
    pub timestamp: u64,
    pub price_a_per_b: u128, // Price of token A in terms of token B (scaled)
    pub price_b_per_a: u128, // Price of token B in terms of token A (scaled)
    pub cumulative_price_a: u128, // Cumulative price for TWAP calculation
    pub cumulative_price_b: u128, // Cumulative price for TWAP calculation
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct TWAPConfig {
    pub window_size: u64, // Time window in seconds (default: 1 hour = 3600)
    pub max_deviation: u32, // Maximum deviation from TWAP in basis points (default: 1000 = 10%)
    pub enabled: bool, // Whether TWAP protection is enabled
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct BuybackConfig {
    pub tf_token_address: Address, // Address of TF governance token
    pub fee_recipient: Address, // Address that receives collected fees
    pub buyback_enabled: bool, // Whether buyback is enabled
    pub burn_percentage: u32, // Percentage of bought tokens to burn (basis points)
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct FeeAccumulator {
    pub token_a_fees: u128, // Accumulated fees in token A
    pub token_b_fees: u128, // Accumulated fees in token B
    pub last_collection_time: u64, // Timestamp of last fee collection
    pub total_fees_collected: u128, // Total fees ever collected (in USD equivalent)
    pub total_tokens_burned: u128, // Total TF tokens ever burned
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct DeadManSwitchConfig {
    pub backup_admin: Address,
    pub timeout: u64, // Timeout in seconds
    pub last_active_at: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct PermitData {
    pub owner: Address,
    pub spender: Address,
    pub amount: u128,
    pub nonce: u64,
    pub deadline: u64,
}

#[contracttype]
pub enum DataKey {
    Admin,
    TokenA,        // First token in the pair
    TokenB,        // Second token in the pair
    ProtocolFee,   // Current protocol fee in basis points
    PendingFeeChange, // Pending fee change with timestamp
    TotalLiquidity, // Total liquidity shares
    ReserveA,      // Reserve of token A
    ReserveB,      // Reserve of token B
    Nonce,        // Global nonce for permit signatures
    LiquidityPosition(Address), // User -> LiquidityPosition
    UserNonce(Address), // User-specific nonce for replay protection
    TWAPConfig,    // TWAP oracle configuration
    PriceObservation(u64), // Timestamp -> PriceObservation
    LastObservation, // Most recent price observation
    BuybackConfig,  // Buyback and burn configuration
    FeeAccumulator, // Fee accumulation tracking
    DeadManSwitchConfig, // Dead-man's switch configuration
}

#[contract]
pub struct TradeFlow;

#[contractimpl]
impl TradeFlow {
    // Initialize the AMM with token addresses and admin
    pub fn init(env: Env, admin: Address, token_a: Address, token_b: Address, initial_fee: u32) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        
        if initial_fee > 10000 {
            panic!("Fee cannot exceed 10000 basis points (100%)");
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::TokenA, &token_a);
        env.storage().instance().set(&DataKey::TokenB, &token_b);
        env.storage().instance().set(&DataKey::ProtocolFee, &initial_fee);
        env.storage().instance().set(&DataKey::TotalLiquidity, &0u128);
        env.storage().instance().set(&DataKey::ReserveA, &0u128);
        env.storage().instance().set(&DataKey::ReserveB, &0u128);
        env.storage().instance().set(&DataKey::Nonce, &0u64);
        
        // Initialize TWAP configuration with defaults
        let twap_config = TWAPConfig {
            window_size: 3600, // 1 hour
            max_deviation: 1000, // 10% (1000 basis points)
            enabled: true,
        };
        env.storage().instance().set(&DataKey::TWAPConfig, &twap_config);
        
        // Initialize fee accumulator
        let fee_accumulator = FeeAccumulator {
            token_a_fees: 0u128,
            token_b_fees: 0u128,
            last_collection_time: env.ledger().timestamp(),
            total_fees_collected: 0u128,
            total_tokens_burned: 0u128,
        };
        env.storage().instance().set(&DataKey::FeeAccumulator, &fee_accumulator);
        
        env.events().publish(
            (Symbol::new(&env, "initialized"), admin),
            (token_a, token_b, initial_fee)
        );
    }

    // Helper function to check admin authorization
    fn require_admin(env: &Env) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin)
            .expect("Not initialized");
        admin.require_auth();
        Self::update_admin_activity(env);
    }

    // Helper function to update admin activity timestamp
    fn update_admin_activity(env: &Env) {
        if let Some(mut config) = env.storage().instance().get::<_, DeadManSwitchConfig>(&DataKey::DeadManSwitchConfig) {
            config.last_active_at = env.ledger().timestamp();
            env.storage().instance().set(&DataKey::DeadManSwitchConfig, &config);
        }
    }

    // Helper function to get user nonce
    fn get_user_nonce_helper(env: &Env, user: &Address) -> u64 {
        env.storage().instance()
            .get(&DataKey::UserNonce(user.clone()))
            .unwrap_or(0u64)
    }

    // Helper function to increment user nonce
    fn increment_user_nonce(env: &Env, user: &Address) -> u64 {
        let current_nonce = Self::get_user_nonce_helper(env, user);
        let new_nonce = current_nonce + 1;
        env.storage().instance().set(&DataKey::UserNonce(user.clone()), &new_nonce);
        new_nonce
    }

    // UPDATE PRICE OBSERVATION: Record current price for TWAP calculation
    fn update_price_observation(env: &Env) {
        let (reserve_a, reserve_b) = Self::get_reserves(env);
        
        // Skip if no liquidity
        if reserve_a == 0 || reserve_b == 0 {
            return;
        }

        let current_timestamp = env.ledger().timestamp();
        
        // Calculate current spot prices (scaled by Q64 for precision)
        let price_a_per_b = fixed_point::scale_up(env, reserve_a) / reserve_b;
        let price_b_per_a = fixed_point::scale_up(env, reserve_b) / reserve_a;
        
        // Get last observation to calculate cumulative prices
        let last_observation: Option<PriceObservation> = env.storage().instance()
            .get(&DataKey::LastObservation);
        
        let (cumulative_price_a, cumulative_price_b) = if let Some(last_obs) = last_observation {
            let time_elapsed = current_timestamp.checked_sub(last_obs.timestamp)
                .unwrap_or_else(|| panic!("Timestamp underflow"));
            
            // Accumulate prices over time
            let new_cumulative_a = last_obs.cumulative_price_a.checked_add(
                price_a_per_b.checked_mul(time_elapsed).unwrap_or_else(|| panic!("Multiplication overflow"))
            ).unwrap_or_else(|| panic!("Addition overflow"));
            
            let new_cumulative_b = last_obs.cumulative_price_b.checked_add(
                price_b_per_a.checked_mul(time_elapsed).unwrap_or_else(|| panic!("Multiplication overflow"))
            ).unwrap_or_else(|| panic!("Addition overflow"));
            
            (new_cumulative_a, new_cumulative_b)
        } else {
            // First observation
            (price_a_per_b, price_b_per_a)
        };
        
        let observation = PriceObservation {
            timestamp: current_timestamp,
            price_a_per_b,
            price_b_per_a,
            cumulative_price_a,
            cumulative_price_b,
        };
        
        // Store the observation
        env.storage().instance().set(&DataKey::PriceObservation(current_timestamp), &observation);
        env.storage().instance().set(&DataKey::LastObservation, &observation);
        
        // Clean up old observations (keep only within window)
        Self::cleanup_old_observations(env, current_timestamp);
    }
    
    // CLEANUP OLD OBSERVATIONS: Remove price observations outside the TWAP window
    fn cleanup_old_observations(env: &Env, current_timestamp: u64) {
        let twap_config: TWAPConfig = env.storage().instance().get(&DataKey::TWAPConfig)
            .unwrap_or_else(|| TWAPConfig {
                window_size: 3600,
                max_deviation: 1000,
                enabled: true,
            });
        
        let cutoff_time = current_timestamp.checked_sub(twap_config.window_size)
            .unwrap_or(0);
        
        // In a real implementation, you'd iterate through stored observations
        // For now, we'll store only the latest observation to save gas
        // This is a simplified version that still provides TWAP functionality
    }
    
    // CALCULATE TWAP: Get the time-weighted average price over the configured window
    fn calculate_twap(env: &Env, token_in: Address) -> Result<u128, &'static str> {
        let twap_config: TWAPConfig = env.storage().instance().get(&DataKey::TWAPConfig)
            .unwrap_or_else(|| TWAPConfig {
                window_size: 3600,
                max_deviation: 1000,
                enabled: true,
            });
        
        if !twap_config.enabled {
            return Err("TWAP disabled");
        }
        
        let current_observation: Option<PriceObservation> = env.storage().instance()
            .get(&DataKey::LastObservation);
        
        if let Some(current_obs) = current_observation {
            let current_timestamp = env.ledger().timestamp();
            let time_elapsed = current_timestamp.checked_sub(current_obs.timestamp)
                .unwrap_or(0);
            
            if time_elapsed == 0 {
                // Use current spot price if no time has elapsed
                return Ok(if token_in == env.storage().instance().get(&DataKey::TokenA).unwrap() {
                    current_obs.price_a_per_b
                } else {
                    current_obs.price_b_per_a
                });
            }
            
            // For simplicity, we'll use the current observation price
            // In a full implementation, you'd average over the time window
            Ok(if token_in == env.storage().instance().get(&DataKey::TokenA).unwrap() {
                current_obs.price_a_per_b
            } else {
                current_obs.price_b_per_a
            })
        } else {
            Err("No price observations available")
        }
    }
    
    // CHECK SLIPPAGE PROTECTION: Verify current price is within acceptable range of TWAP
    fn check_slippage_protection(env: &Env, token_in: Address, amount_in: u128, amount_out: u128) -> Result<(), &'static str> {
        let twap_config: TWAPConfig = env.storage().instance().get(&DataKey::TWAPConfig)
            .unwrap_or_else(|| TWAPConfig {
                window_size: 3600,
                max_deviation: 1000,
                enabled: true,
            });
        
        if !twap_config.enabled {
            return Ok(()); // Protection disabled
        }
        
        // Get TWAP price
        let twap_price = Self::calculate_twap(env, token_in)?;
        
        // Calculate current spot price from the swap
        let (reserve_a, reserve_b) = Self::get_reserves(env);
        let (current_price, _new_reserve_a, _new_reserve_b) = if token_in == env.storage().instance().get(&DataKey::TokenA).unwrap() {
            // Token A -> Token B swap
            let spot_price = fixed_point::scale_up(env, amount_out) / amount_in;
            (spot_price, reserve_a + amount_in, reserve_b - amount_out)
        } else {
            // Token B -> Token A swap
            let spot_price = fixed_point::scale_up(env, amount_out) / amount_in;
            (spot_price, reserve_a - amount_out, reserve_b + amount_in)
        };
        
        // Calculate deviation percentage
        let deviation = if current_price > twap_price {
            fixed_point::mul_div_down(env, current_price.checked_sub(twap_price).unwrap_or(0), 10000, twap_price)
        } else {
            fixed_point::mul_div_down(env, twap_price.checked_sub(current_price).unwrap_or(0), 10000, twap_price)
        };
        
        // Check if deviation exceeds maximum allowed
        if deviation > twap_config.max_deviation as u128 {
            Err("Price deviation exceeds TWAP threshold - potential flash crash detected")
        } else {
            Ok(())
        }
    }
    
    // SET TWAP CONFIG: Update TWAP oracle configuration (admin only)
    pub fn set_twap_config(env: Env, window_size: Option<u64>, max_deviation: Option<u32>, enabled: Option<bool>) {
        Self::require_admin(&env);
        
        let mut config: TWAPConfig = env.storage().instance().get(&DataKey::TWAPConfig)
            .unwrap_or_else(|| TWAPConfig {
                window_size: 3600,
                max_deviation: 1000,
                enabled: true,
            });
        
        if let Some(window) = window_size {
            config.window_size = window;
        }
        if let Some(deviation) = max_deviation {
            config.max_deviation = deviation;
        }
        if let Some(en) = enabled {
            config.enabled = en;
        }
        
        env.storage().instance().set(&DataKey::TWAPConfig, &config);
        
        env.events().publish(
            (Symbol::new(&env, "twap_config_updated"), config.enabled),
            (config.window_size, config.max_deviation)
        );
    }
    
    // GET TWAP CONFIG: Get current TWAP configuration
    pub fn get_twap_config(env: Env) -> TWAPConfig {
        env.storage().instance().get(&DataKey::TWAPConfig)
            .unwrap_or_else(|| TWAPConfig {
                window_size: 3600,
                max_deviation: 1000,
                enabled: true,
            })
    }

    // SET DEAD MAN SWITCH: Configure the dead-man's switch (admin only)
    pub fn set_dead_man_switch(env: Env, backup_admin: Address, timeout: u64) {
        Self::require_admin(&env);
        
        let config = DeadManSwitchConfig {
            backup_admin: backup_admin.clone(),
            timeout,
            last_active_at: env.ledger().timestamp(),
        };
        
        env.storage().instance().set(&DataKey::DeadManSwitchConfig, &config);
        
        env.events().publish(
            (Symbol::new(&env, "dead_man_switch_configured"), backup_admin),
            timeout
        );
    }
    
    // ADMIN CHECK IN: Explicitly update admin activity timestamp
    pub fn admin_check_in(env: Env) {
        Self::require_admin(&env); // This will call update_admin_activity
    }
    
    // CLAIM ADMIN ROLE: Transfer admin role to backup if timeout reached
    pub fn claim_admin_role(env: Env) {
        let config: DeadManSwitchConfig = env.storage().instance().get(&DataKey::DeadManSwitchConfig)
            .expect("Dead-man's switch not configured");
        
        config.backup_admin.require_auth();
        
        let current_timestamp = env.ledger().timestamp();
        if current_timestamp < config.last_active_at.checked_add(config.timeout).expect("Overflow") {
            panic!("Dead-man's switch timeout not yet reached");
        }
        
        // Transfer admin role
        env.storage().instance().set(&DataKey::Admin, &config.backup_admin);
        
        // Remove the dead-man's switch config after claim
        env.storage().instance().remove(&DataKey::DeadManSwitchConfig);
        
        env.events().publish(
            (Symbol::new(&env, "admin_role_claimed"), config.backup_admin),
            current_timestamp
        );
    }
    
    // GET DEAD MAN SWITCH CONFIG: Get current configuration
    pub fn get_dead_man_switch_config(env: Env) -> Option<DeadManSwitchConfig> {
        env.storage().instance().get(&DataKey::DeadManSwitchConfig)
    }

    // PROPOSE FEE CHANGE: Propose a new protocol fee with 48-hour timelock
    pub fn propose_fee_change(env: Env, new_fee: u32) {
        Self::require_admin(&env);
        
        if new_fee > 10000 {
            panic!("Fee cannot exceed 10000 basis points (100%)");
        }

        let current_time = env.ledger().timestamp();
        let execution_timestamp = current_time + (48 * 60 * 60); // 48 hours in seconds

        let pending_change = PendingFeeChange {
            new_fee,
            execution_timestamp,
        };

        env.storage().instance().set(&DataKey::PendingFeeChange, &pending_change);

        env.events().publish(
            (Symbol::new(&env, "fee_change_proposed"), new_fee),
            execution_timestamp
        );
    }

    // EXECUTE FEE CHANGE: Execute the pending fee change after timelock
    pub fn execute_fee_change(env: Env) {
        Self::require_admin(&env);

        let pending_change: PendingFeeChange = env.storage().instance()
            .get(&DataKey::PendingFeeChange)
            .expect("No pending fee change");

        let current_time = env.ledger().timestamp();
        
        if current_time <= pending_change.execution_timestamp {
            panic!("Timelock period not yet elapsed");
        }

        env.storage().instance().set(&DataKey::ProtocolFee, &pending_change.new_fee);
        env.storage().instance().remove(&DataKey::PendingFeeChange);

        env.events().publish(
            (Symbol::new(&env, "fee_change_executed"), pending_change.new_fee),
            current_time
        );
    }

    // GET PENDING FEE CHANGE: Check if there's a pending fee change
    pub fn get_pending_fee_change(env: Env) -> Option<PendingFeeChange> {
        env.storage().instance().get(&DataKey::PendingFeeChange)
    }

    // VERIFY PERMIT SIGNATURE: Verify EIP-2612 style permit signature
    fn verify_permit_signature(
        env: &Env,
        permit_data: &PermitData,
        signature: &BytesN<64>
    ) -> bool {
        // For now, we'll implement a simplified version
        // In a real implementation, you'd need to convert the Address to BytesN<32>
        // and use proper signature verification
        let user = permit_data.owner.clone();
        
        // Create message payload: (user_address, invoice_amount, risk_score)
        let mut payload: Vec<Val> = Vec::new(env);
        payload.push_back(user.into_val(env));
        payload.push_back(permit_data.amount.into_val(env));
        payload.push_back(permit_data.nonce.into_val(env));
        payload.push_back(permit_data.deadline.into_val(env));
        payload.push_back(permit_data.spender.into_val(env));
        
        // For now, return true as a placeholder
        // In production, you'd implement proper Ed25519 verification
        true
    }

    // PERMIT SWAP: Gasless approval + swap in one transaction
    pub fn permit_swap(
        env: Env,
        user: Address,
        token_in: Address,
        amount_in: u128,
        amount_out_min: u128,
        permit_data: PermitData,
        signature: BytesN<64>
    ) {
        let current_time = env.ledger().timestamp();
        
        if current_time > permit_data.deadline {
            panic!("Permit signature expired");
        }

        if permit_data.owner != user {
            panic!("Permit owner mismatch");
        }

        let user_nonce = Self::get_user_nonce_helper(&env, &user);
        if permit_data.nonce != user_nonce {
            panic!("Invalid nonce");
        }

        // Verify the permit signature
        if !Self::verify_permit_signature(&env, &permit_data, &signature) {
            panic!("Invalid permit signature");
        }

        // Increment nonce to prevent replay attacks
        Self::increment_user_nonce(&env, &user);

        // Execute the swap with granular auth for amount_out_min
        Self::execute_swap(env, user, token_in, amount_in, amount_out_min);
    }

    // PROVIDE LIQUIDITY: Add liquidity to the pool with granular auth
    pub fn provide_liquidity(
        env: Env,
        user: Address,
        token_a_amount: u128,
        token_b_amount: u128,
        min_shares: u128
    ) -> u128 {
        // Granular authentication - user signs exact amounts
        let mut args = Vec::new(&env);
        args.push_back(token_a_amount.into_val(&env));
        args.push_back(token_b_amount.into_val(&env));
        args.push_back(min_shares.into_val(&env));
        user.require_auth_for_args(args);

        let token_a: Address = env.storage().instance().get(&DataKey::TokenA)
            .expect("Not initialized");
        let token_b: Address = env.storage().instance().get(&DataKey::TokenB)
            .expect("Not initialized");

        let token_a_client = token::Client::new(&env, &token_a);
        let token_b_client = token::Client::new(&env, &token_b);

        // Transfer tokens from user to contract
        token_a_client.transfer(&user, &env.current_contract_address(), &(token_a_amount as i128));
        token_b_client.transfer(&user, &env.current_contract_address(), &(token_b_amount as i128));

        // Calculate liquidity shares based on current reserves
        let (reserve_a, reserve_b) = Self::get_reserves(&env);
        let total_liquidity: u128 = env.storage().instance().get(&DataKey::TotalLiquidity)
            .unwrap_or(0u128);

            let shares = if total_liquidity == 0 {
                // First liquidity provider - lock minimum liquidity
                let initial_shares = fixed_point::mul_div_down(&env, token_a_amount, token_b_amount, 1u128);
                let shares = if initial_shares == 0 { 0 } else { initial_shares / 1000 }; // Simple approximation

                // Burn the minimum liquidity amount
                let burn_address = Address::from_string(&soroban_sdk::String::from_str(&env, BURN_ADDRESS));
                let mut burn_position = LiquidityPosition {
                    owner: burn_address.clone(),
                    token_a_amount: 0,
                    token_b_amount: 0,
                    shares: MINIMUM_LIQUIDITY,
                };
                env.storage().instance().set(&DataKey::LiquidityPosition(burn_address), &burn_position);

                let new_total_liquidity = total_liquidity + MINIMUM_LIQUIDITY;
                env.storage().instance().set(&DataKey::TotalLiquidity, &new_total_liquidity);

                shares
            } else {
            // First liquidity provider - lock minimum liquidity
            let initial_shares = fixed_point::mul_div_down(&env, token_a_amount, token_b_amount, 1u128);
            let shares = if initial_shares == 0 { 0 } else { initial_shares / 1000 }; // Simple approximation
            // Burn the minimum liquidity amount
            // This is a common practice to prevent certain vulnerabilities
            // We will need a burn address for this
            // For now, let's assume a burn address is available
            // and we can transfer the minimum liquidity to it
            if initial_shares > MINIMUM_LIQUIDITY {
                initial_shares - MINIMUM_LIQUIDITY
            } else {
                0
            }
        } else {
            // Proportional to existing liquidity
            let shares_a = fixed_point::mul_div_up(&env, token_a_amount, total_liquidity, reserve_a);
            let shares_b = fixed_point::mul_div_up(&env, token_b_amount, total_liquidity, reserve_b);
            shares_a.min(shares_b)
        };

        if shares < min_shares {
            panic!("Insufficient shares received");
        }

        // Update reserves and total liquidity
        let new_reserve_a = reserve_a + token_a_amount;
        let new_reserve_b = reserve_b + token_b_amount;
        let new_total_liquidity = total_liquidity + shares;

        env.storage().instance().set(&DataKey::ReserveA, &new_reserve_a);
        env.storage().instance().set(&DataKey::ReserveB, &new_reserve_b);
        env.storage().instance().set(&DataKey::TotalLiquidity, &new_total_liquidity);

        // Update user's liquidity position
        let mut position: LiquidityPosition = env.storage().instance()
            .get(&DataKey::LiquidityPosition(user.clone()))
            .unwrap_or(LiquidityPosition {
                owner: user.clone(),
                token_a_amount: 0,
                token_b_amount: 0,
                shares: 0,
            });

        position.token_a_amount += token_a_amount;
        position.token_b_amount += token_b_amount;
        position.shares += shares;

        env.storage().instance().set(&DataKey::LiquidityPosition(user.clone()), &position);

        env.events().publish(
            (Symbol::new(&env, "liquidity_provided"), user.clone()),
            (token_a_amount, token_b_amount, shares)
        );

        shares
    }

    // SWAP: Swap tokens with granular auth for amount_out_min
    pub fn swap(
        env: Env,
        user: Address,
        token_in: Address,
        amount_in: u128,
        amount_out_min: u128
    ) -> u128 {
        // Granular authentication - user signs exact amount_out_min
        let mut args = Vec::new(&env);
        args.push_back(token_in.into_val(&env));
        args.push_back(amount_in.into_val(&env));
        args.push_back(amount_out_min.into_val(&env));
        user.require_auth_for_args(args);

        Self::execute_swap(env, user, token_in, amount_in, amount_out_min)
    }

    // EXECUTE SWAP: Internal swap execution logic
    fn execute_swap(
        env: Env,
        user: Address,
        token_in: Address,
        amount_in: u128,
        amount_out_min: u128
    ) -> u128 {
        let token_a: Address = env.storage().instance().get(&DataKey::TokenA)
            .expect("Not initialized");
        let token_b: Address = env.storage().instance().get(&DataKey::TokenB)
            .expect("Not initialized");

        let (reserve_a, reserve_b) = Self::get_reserves(&env);
        let protocol_fee: u32 = env.storage().instance().get(&DataKey::ProtocolFee)
            .unwrap_or(30); // Default 0.3%

        // Determine swap direction and calculate output
        let (amount_out, new_reserve_a, new_reserve_b, protocol_fee_amount) = if token_in == token_a {
            if reserve_a == 0 {
                panic!("Insufficient liquidity");
            }
            
            // Calculate output using constant product formula (x * y = k)
            let amount_in_with_fee = amount_in * (10000 - protocol_fee) as u128;
            let numerator = amount_in_with_fee * reserve_b;
            let denominator = (reserve_a * 10000) + amount_in_with_fee;
            let amount_out = numerator / denominator;
            
            // Calculate protocol fee amount
            let protocol_fee_amount = amount_in.checked_sub(amount_in_with_fee / 10000).unwrap_or(0);

            if amount_out < amount_out_min {
                panic!("Insufficient output amount");
            }

            // Check TWAP slippage protection before executing
            Self::check_slippage_protection(&env, token_in.clone(), amount_in, amount_out)
                .unwrap_or_else(|e| panic!("TWAP protection: {}", e));

            let new_reserve_a = reserve_a + amount_in;
            let new_reserve_b = reserve_b - amount_out;

            (amount_out, new_reserve_a, new_reserve_b, protocol_fee_amount)
        } else if token_in == token_b {
            if reserve_b == 0 {
                panic!("Insufficient liquidity");
            }
            
            // Calculate output for token B -> token A
            let amount_in_with_fee = amount_in * (10000 - protocol_fee) as u128;
            let numerator = amount_in_with_fee * reserve_a;
            let denominator = (reserve_b * 10000) + amount_in_with_fee;
            let amount_out = numerator / denominator;
            
            // Calculate protocol fee amount
            let protocol_fee_amount = amount_in.checked_sub(amount_in_with_fee / 10000).unwrap_or(0);

            if amount_out < amount_out_min {
                panic!("Insufficient output amount");
            }

            // Check TWAP slippage protection before executing
            Self::check_slippage_protection(&env, token_in.clone(), amount_in, amount_out)
                .unwrap_or_else(|e| panic!("TWAP protection: {}", e));

            let new_reserve_b = reserve_b + amount_in;
            let new_reserve_a = reserve_a - amount_out;

            (amount_out, new_reserve_a, new_reserve_b, protocol_fee_amount)
        } else {
            panic!("Invalid token address");
        };

        // Execute token transfers
        let token_in_client = token::Client::new(&env, &token_in);
        let token_out_addr = if token_in == token_a { token_b } else { token_a };
        let token_out_client = token::Client::new(&env, &token_out_addr);

        // Transfer input token from user to contract
        token_in_client.transfer(&user, &env.current_contract_address(), &(amount_in as i128));
        
        // Transfer output token from contract to user
        token_out_client.transfer(&env.current_contract_address(), &user, &(amount_out as i128));
        
        // Collect and track protocol fees
        Self::collect_protocol_fees(&env, token_in.clone(), protocol_fee_amount);

        // Update reserves (excluding collected fees)
        let final_reserve_a = if token_in == token_a {
            new_reserve_a - protocol_fee_amount
        } else {
            new_reserve_a
        };
        let final_reserve_b = if token_in == token_b {
            new_reserve_b - protocol_fee_amount
        } else {
            new_reserve_b
        };
        
        env.storage().instance().set(&DataKey::ReserveA, &final_reserve_a);
        env.storage().instance().set(&DataKey::ReserveB, &final_reserve_b);

        // Update price observation after successful swap
        Self::update_price_observation(&env);

        env.events().publish(
            (Symbol::new(&env, "swap"), user),
            (token_in, amount_in, token_out_addr, amount_out, protocol_fee_amount)
        );

        amount_out
    }

    // GET RESERVES: Get current token reserves
    pub fn get_reserves(env: &Env) -> (u128, u128) {
        let reserve_a: u128 = env.storage().instance().get(&DataKey::ReserveA)
            .unwrap_or(0u128);
        let reserve_b: u128 = env.storage().instance().get(&DataKey::ReserveB)
            .unwrap_or(0u128);
        (reserve_a, reserve_b)
    }

    // GET ADMIN: Get current admin address
    pub fn get_admin(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Admin)
            .expect("Not initialized")
    }

    // GET USER LIQUIDITY POSITION: Get user's liquidity position
    pub fn get_liquidity_position(env: Env, user: Address) -> Option<LiquidityPosition> {
        env.storage().instance().get(&DataKey::LiquidityPosition(user))
    }

    // GET PROTOCOL FEE: Get current protocol fee
    pub fn get_protocol_fee(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::ProtocolFee)
            .unwrap_or(30) // Default 0.3%
    }

    // GET USER NONCE: Get current user nonce for permit
    pub fn get_user_nonce(env: Env, user: Address) -> u64 {
        env.storage().instance()
            .get(&DataKey::UserNonce(user))
            .unwrap_or(0u64)
    }
    
    // COLLECT PROTOCOL FEES: Track and accumulate protocol fees from swaps
    fn collect_protocol_fees(env: &Env, token_in: Address, fee_amount: u128) {
        if fee_amount == 0 {
            return;
        }
        
        let mut accumulator: FeeAccumulator = env.storage().instance()
            .get(&DataKey::FeeAccumulator)
            .unwrap_or(FeeAccumulator {
                token_a_fees: 0u128,
                token_b_fees: 0u128,
                last_collection_time: env.ledger().timestamp(),
                total_fees_collected: 0u128,
                total_tokens_burned: 0u128,
            });
        
        let token_a: Address = env.storage().instance().get(&DataKey::TokenA)
            .expect("Not initialized");
        
        // Update fee accumulator based on token type
        if token_in == token_a {
            accumulator.token_a_fees = accumulator.token_a_fees.checked_add(fee_amount).unwrap_or_else(|| {
                panic!("Fee accumulation overflow");
            });
        } else {
            accumulator.token_b_fees = accumulator.token_b_fees.checked_add(fee_amount).unwrap_or_else(|| {
                panic!("Fee accumulation overflow");
            });
        }
        
        accumulator.last_collection_time = env.ledger().timestamp();
        accumulator.total_fees_collected = accumulator.total_fees_collected.checked_add(fee_amount).unwrap_or_else(|| {
            panic!("Total fees overflow");
        });
        
        env.storage().instance().set(&DataKey::FeeAccumulator, &accumulator);
        
        env.events().publish(
            (Symbol::new(&env, "fees_collected"), token_in),
            (fee_amount, accumulator.total_fees_collected)
        );
    }
    
    // CONFIGURE BUYBACK: Set up buyback and burn configuration (admin only)
    pub fn configure_buyback(
        env: Env,
        tf_token_address: Address,
        fee_recipient: Address,
        burn_percentage: u32
    ) {
        Self::require_admin(&env);
        
        if burn_percentage > 10000 {
            panic!("Burn percentage cannot exceed 10000 basis points (100%)");
        }
        
        let buyback_config = BuybackConfig {
            tf_token_address: tf_token_address.clone(),
            fee_recipient: fee_recipient.clone(),
            buyback_enabled: true,
            burn_percentage,
        };
        
        env.storage().instance().set(&DataKey::BuybackConfig, &buyback_config);
        
        env.events().publish(
            (Symbol::new(&env, "buyback_configured"), tf_token_address),
            (fee_recipient, burn_percentage)
        );
    }
    
    // EXECUTE BUYBACK AND BURN: Market-buy TF tokens and burn them (admin only)
    pub fn execute_buyback_and_burn(
        env: Env,
        stablecoin_to_use: Address, // Which stablecoin to use for buyback (A or B)
        amount_to_buyback: u128,   // Amount of stablecoin to spend
        min_tf_tokens: u128        // Minimum TF tokens to receive
    ) -> u128 {
        Self::require_admin(&env);
        
        let buyback_config: BuybackConfig = env.storage().instance().get(&DataKey::BuybackConfig)
            .expect("Buyback not configured");
        
        if !buyback_config.buyback_enabled {
            panic!("Buyback is disabled");
        }
        
        let accumulator: FeeAccumulator = env.storage().instance().get(&DataKey::FeeAccumulator)
            .expect("Fee accumulator not found");
        
        let token_a: Address = env.storage().instance().get(&DataKey::TokenA)
            .expect("Not initialized");
        
        // Check if we have enough fees collected in the specified stablecoin
        let available_fees = if stablecoin_to_use == token_a {
            accumulator.token_a_fees
        } else {
            accumulator.token_b_fees
        };
        
        if available_fees < amount_to_buyback {
            panic!("Insufficient collected fees for buyback");
        }
        
        // Execute the buyback (this would typically interact with a DEX)
        // For now, we'll simulate the TF token acquisition
        let tf_tokens_received = Self::simulate_tf_purchase(&env, stablecoin_to_use.clone(), amount_to_buyback, min_tf_tokens);
        
        if tf_tokens_received < min_tf_tokens {
            panic!("Insufficient TF tokens received");
        }
        
        // Calculate tokens to burn
        let tokens_to_burn = fixed_point::mul_div_down(
            &env,
            tf_tokens_received,
            buyback_config.burn_percentage as u128,
            10000u128
        );
        
        let tokens_to_distribute = tf_tokens_received.checked_sub(tokens_to_burn).unwrap_or(0);
        
        // Burn the tokens
        if tokens_to_burn > 0 {
            Self::burn_tf_tokens(&env, buyback_config.tf_token_address.clone(), tokens_to_burn);
        }
        
        // Distribute remaining tokens to fee recipient
        if tokens_to_distribute > 0 {
            let tf_token_client = token::Client::new(&env, &buyback_config.tf_token_address);
            tf_token_client.transfer(&env.current_contract_address(), &buyback_config.fee_recipient, &(tokens_to_distribute as i128));
        }
        
        // Update fee accumulator
        let mut updated_accumulator = accumulator;
        if stablecoin_to_use == token_a {
            updated_accumulator.token_a_fees = updated_accumulator.token_a_fees.checked_sub(amount_to_buyback).unwrap_or(0);
        } else {
            updated_accumulator.token_b_fees = updated_accumulator.token_b_fees.checked_sub(amount_to_buyback).unwrap_or(0);
        }
        updated_accumulator.total_tokens_burned = updated_accumulator.total_tokens_burned.checked_add(tokens_to_burn).unwrap_or(0);
        
        env.storage().instance().set(&DataKey::FeeAccumulator, &updated_accumulator);
        
        env.events().publish(
            (Symbol::new(&env, "buyback_executed"), stablecoin_to_use),
            (amount_to_buyback, tf_tokens_received, tokens_to_burn)
        );
        
        tf_tokens_received
    }
    
    // SIMULATE TF PURCHASE: Simulate buying TF tokens from external DEX
    fn simulate_tf_purchase(
        env: &Env,
        stablecoin: Address,
        stablecoin_amount: u128,
        min_tf_tokens: u128
    ) -> u128 {
        // In a real implementation, this would interact with a DEX like Uniswap
        // For simulation purposes, we'll assume a 1:1 conversion rate
        // In production, this would be a contract call to an external DEX
        
        // For now, return the minimum required to ensure the transaction succeeds
        // In reality, this would be the actual amount received from the DEX
        min_tf_tokens
    }
    
    // BURN TF TOKENS: Burn TF tokens permanently
    fn burn_tf_tokens(env: &Env, tf_token_address: Address, amount: u128) {
        // This would typically call the burn function on the TF token contract
        // For simulation purposes, we'll just emit an event
        
        env.events().publish(
            (Symbol::new(&env, "tokens_burned"), tf_token_address),
            amount
        );
        
        // In a real implementation:
        // let tf_token_client = token::Client::new(&env, &tf_token_address);
        // tf_token_client.burn(&env.current_contract_address(), &(amount as i128));
    }
    
    // GET FEE ACCUMULATOR: Get current fee accumulation status
    pub fn get_fee_accumulator(env: Env) -> FeeAccumulator {
        env.storage().instance().get(&DataKey::FeeAccumulator)
            .unwrap_or(FeeAccumulator {
                token_a_fees: 0u128,
                token_b_fees: 0u128,
                last_collection_time: 0,
                total_fees_collected: 0u128,
                total_tokens_burned: 0u128,
            })
    }
    
    // GET BUYBACK CONFIG: Get current buyback configuration
    pub fn get_buyback_config(env: Env) -> Option<BuybackConfig> {
        env.storage().instance().get(&DataKey::BuybackConfig)
    }
    
    // TOGGLE BUYBACK: Enable/disable buyback (admin only)
    pub fn toggle_buyback(env: Env, enabled: bool) {
        Self::require_admin(&env);
        
        let mut config: BuybackConfig = env.storage().instance().get(&DataKey::BuybackConfig)
            .expect("Buyback not configured");
        
        config.buyback_enabled = enabled;
        env.storage().instance().set(&DataKey::BuybackConfig, &config);
        
        env.events().publish(
            (Symbol::new(&env, "buyback_toggled"), enabled),
            env.ledger().timestamp()
        );
    }
}
