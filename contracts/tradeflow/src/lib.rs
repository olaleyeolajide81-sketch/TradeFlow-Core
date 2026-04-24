#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, 
    token, Address, Env, Symbol, Map, BytesN, Vec, Val, IntoVal, Bytes
};

mod utils;
use utils::fixed_point::{self, Q64};

mod error;
use error::{Error, check_and_panic_error};

#[cfg(test)]
mod tests;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiquidityPosition {
    pub owner: Address,
    pub token_a_amount: u128,
    pub token_b_amount: u128,
    pub shares: u128,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct PendingFeeChange {
    pub new_fee: u32, // Fee in basis points (100 = 1%)
    pub execution_timestamp: u64,
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
    MaxTradePercentage, // Maximum trade size as percentage of pool reserves
    FeeRecipient, // Address that receives protocol fees
    FlashLoanActive, // Flash loan reentrancy lock
    FactoryPaused, // Factory-level emergency pause state
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
        env.storage().instance().set(&DataKey::MaxTradePercentage, &10u32); // Default 10%
        env.storage().instance().set(&DataKey::FeeRecipient, &admin); // Default to admin
        env.storage().instance().set(&DataKey::FlashLoanActive, &false);
        env.storage().instance().set(&DataKey::FactoryPaused, &false);
        
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
    }

    // Helper function to check if factory is active
    fn require_factory_active(env: &Env) {
        let factory_paused: bool = env.storage().instance().get(&DataKey::FactoryPaused)
            .unwrap_or(false);
        if factory_paused {
            check_and_panic_error(Error::FactoryPaused);
        }
    }

    // Helper function to check token allowance
    fn check_allowance(env: &Env, user: &Address, token: &Address, spender: &Address, amount: u128) {
        let token_client = token::Client::new(env, token);
        let allowance = token_client.allowance(user, spender);
        
        if allowance < amount as i128 {
            check_and_panic_error(Error::InsufficientAllowance);
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

    // UPDATE MAX TRADE SIZE: Admin function to update maximum trade percentage
    pub fn update_max_trade_size(env: Env, new_percentage: u32) {
        Self::require_admin(&env);
        
        if new_percentage > 50 {
            check_and_panic_error(Error::TradeSizeExceedsMaximum);
        }

        let old_percentage: u32 = env.storage().instance().get(&DataKey::MaxTradePercentage)
            .unwrap_or(10u32);
        
        env.storage().instance().set(&DataKey::MaxTradePercentage, &new_percentage);

        env.events().publish(
            (Symbol::new(&env, "max_trade_size_updated"), old_percentage),
            new_percentage
        );
    }

    // UPDATE FEE RECIPIENT: Admin function to update protocol fee recipient
    pub fn update_fee_recipient(env: Env, new_recipient: Address) {
        Self::require_admin(&env);

        let old_recipient: Address = env.storage().instance().get(&DataKey::FeeRecipient)
            .expect("Not initialized");
        
        env.storage().instance().set(&DataKey::FeeRecipient, &new_recipient);

        env.events().publish(
            (Symbol::new(&env, "fee_recipient_changed"), old_recipient),
            new_recipient
        );
    }

    // SET FACTORY PAUSE STATE: Admin function to pause/unpause the entire factory
    pub fn set_factory_pause_state(env: Env, state: bool) {
        Self::require_admin(&env);

        let old_state: bool = env.storage().instance().get(&DataKey::FactoryPaused)
            .unwrap_or(false);
        
        env.storage().instance().set(&DataKey::FactoryPaused, &state);

        env.events().publish(
            (Symbol::new(&env, "factory_pause_state_changed"), old_state),
            state
        );
    }

    // GET FACTORY PAUSE STATE: Get current factory pause state
    pub fn get_factory_pause_state(env: Env) -> bool {
        env.storage().instance().get(&DataKey::FactoryPaused)
            .unwrap_or(false)
    }

    // SWEEP TOKENS: Admin function to rescue accidentally transferred tokens
    pub fn sweep_tokens(env: Env, token: Address, to: Address) {
        Self::require_admin(&env);

        // Get the token addresses from the pool
        let token_a: Address = env.storage().instance().get(&DataKey::TokenA)
            .expect("Not initialized");
        let token_b: Address = env.storage().instance().get(&DataKey::TokenB)
            .expect("Not initialized");

        // Verify the token is one of the pool tokens
        if token != token_a && token != token_b {
            check_and_panic_error(Error::InvalidTokenAddress);
        }

        // Get the mathematical reserve from pool state
        let (reserve_a, reserve_b) = Self::get_reserves(&env);
        let reserve_amount = if token == token_a { reserve_a } else { reserve_b };

        // Get the actual contract balance
        let token_client = token::Client::new(&env, &token);
        let actual_balance = token_client.balance(&env.current_contract_address()) as u128;

        // Calculate the difference (actual_balance - reserve)
        if actual_balance <= reserve_amount {
            check_and_panic_error(Error::InsufficientBalance);
        }

        let sweep_amount = actual_balance - reserve_amount;

        // Transfer the excess tokens to the specified recovery address
        token_client.transfer(&env.current_contract_address(), &to, &(sweep_amount as i128));

        env.events().publish(
            (Symbol::new(&env, "tokens_swept"), token.clone()),
            (to, sweep_amount)
        );
    }

    // GET MAX TRADE SIZE: Get current maximum trade percentage
    pub fn get_max_trade_size(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::MaxTradePercentage)
            .unwrap_or(10u32) // Default 10%
    }

    // GET FEE RECIPIENT: Get current fee recipient address
    pub fn get_fee_recipient(env: Env) -> Address {
        env.storage().instance().get(&DataKey::FeeRecipient)
            .expect("Not initialized")
    }

    // VERIFY PERMIT SIGNATURE: Verify EIP-2612 style permit signature
    fn verify_permit_signature(
        env: &Env,
        permit_data: &PermitData,
        _signature: &BytesN<64>
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
        // Check if factory is active
        Self::require_factory_active(&env);
        
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
        // Check if factory is active
        Self::require_factory_active(&env);
        
        // Check flash loan reentrancy lock
        let flash_loan_active: bool = env.storage().temporary().get(&DataKey::FlashLoanActive)
            .unwrap_or(false);
        if flash_loan_active {
            check_and_panic_error(Error::FlashLoanActive);
        }

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

        // Check token allowances before attempting transfers
        let contract_address = env.current_contract_address();
        Self::check_allowance(&env, &user, &token_a, &contract_address, token_a_amount);
        Self::check_allowance(&env, &user, &token_b, &contract_address, token_b_amount);

        // Transfer tokens from user to contract
        token_a_client.transfer(&user, &env.current_contract_address(), &(token_a_amount as i128));
        token_b_client.transfer(&user, &env.current_contract_address(), &(token_b_amount as i128));

        // Calculate liquidity shares based on current reserves
        let (reserve_a, reserve_b) = Self::get_reserves(&env);
        let total_liquidity: u128 = env.storage().instance().get(&DataKey::TotalLiquidity)
            .unwrap_or(0u128);

        let shares = if total_liquidity == 0 {
            // First liquidity provider - use geometric mean approximation
            // Since we don't have sqrt, use a simpler approach
            let product = fixed_point::mul_div_down(&env, token_a_amount, token_b_amount, 1u128);
            if product == 0 { 1000 } else { product / 1000 } // Simple approximation
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
        // Check if factory is active
        Self::require_factory_active(&env);
        
        // Check flash loan reentrancy lock
        let flash_loan_active: bool = env.storage().temporary().get(&DataKey::FlashLoanActive)
            .unwrap_or(false);
        if flash_loan_active {
            check_and_panic_error(Error::FlashLoanActive);
        }

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

        // Check trade size against maximum allowed percentage
        let max_trade_percentage: u32 = env.storage().instance().get(&DataKey::MaxTradePercentage)
            .unwrap_or(10u32); // Default 10%
        
        let (_reserve_for_token, max_allowed) = if token_in == token_a {
            (reserve_a, (reserve_a * max_trade_percentage as u128) / 100u128)
        } else {
            (reserve_b, (reserve_b * max_trade_percentage as u128) / 100u128)
        };

        if amount_in > max_allowed {
            check_and_panic_error(Error::TradeSizeExceedsMaximum);
        }

        // Determine swap direction and calculate output
        let (amount_out, new_reserve_a, new_reserve_b) = if token_in == token_a {
            if reserve_a == 0 {
                panic!("Insufficient liquidity");
            }
            
            // Calculate output using constant product formula (x * y = k)
            let amount_in_with_fee = amount_in * (10000 - protocol_fee) as u128;
            let numerator = amount_in_with_fee * reserve_b;
            let denominator = (reserve_a * 10000) + amount_in_with_fee;
            let amount_out = numerator / denominator;

            if amount_out < amount_out_min {
                panic!("Insufficient output amount");
            }

            let new_reserve_a = reserve_a + amount_in;
            let new_reserve_b = reserve_b - amount_out;

            (amount_out, new_reserve_a, new_reserve_b)
        } else if token_in == token_b {
            if reserve_b == 0 {
                panic!("Insufficient liquidity");
            }
            
            // Calculate output for token B -> token A
            let amount_in_with_fee = amount_in * (10000 - protocol_fee) as u128;
            let numerator = amount_in_with_fee * reserve_a;
            let denominator = (reserve_b * 10000) + amount_in_with_fee;
            let amount_out = numerator / denominator;

            if amount_out < amount_out_min {
                panic!("Insufficient output amount");
            }

            let new_reserve_b = reserve_b + amount_in;
            let new_reserve_a = reserve_a - amount_out;

            (amount_out, new_reserve_a, new_reserve_b)
        } else {
            panic!("Invalid token address");
        };

        // Execute token transfers
        let token_in_client = token::Client::new(&env, &token_in);
        let token_out_addr = if token_in == token_a { token_b } else { token_a };
        let token_out_client = token::Client::new(&env, &token_out_addr);

        // Check token allowance before attempting transfer
        let contract_address = env.current_contract_address();
        Self::check_allowance(&env, &user, &token_in, &contract_address, amount_in);

        // Transfer input token from user to contract
        token_in_client.transfer(&user, &env.current_contract_address(), &(amount_in as i128));
        
        // Transfer output token from contract to user
        token_out_client.transfer(&env.current_contract_address(), &user, &(amount_out as i128));

        // Update reserves
        env.storage().instance().set(&DataKey::ReserveA, &new_reserve_a);
        env.storage().instance().set(&DataKey::ReserveB, &new_reserve_b);

        env.events().publish(
            (Symbol::new(&env, "swap"), user),
            (token_in, amount_in, token_out_addr, amount_out)
        );

        amount_out
    }

    // ESTIMATE LP TOKENS: View function to estimate LP tokens for a deposit
    pub fn estimate_lp_tokens(env: Env, amount_a: u128, amount_b: u128) -> u128 {
        let (reserve_a, reserve_b) = Self::get_reserves(&env);
        let total_liquidity: u128 = env.storage().instance().get(&DataKey::TotalLiquidity)
            .unwrap_or(0u128);

        if total_liquidity == 0 {
            // First liquidity provider - use geometric mean formula
            // Since we don't have sqrt, use approximation: sqrt(amount_a * amount_b)
            let product = fixed_point::mul_div_down(&env, amount_a, amount_b, 1u128);
            if product == 0 { 
                1000 // Minimum shares for first deposit
            } else { 
                // Simple approximation of square root using division
                // This is a conservative estimate for the geometric mean
                let min_amount = amount_a.min(amount_b);
                let max_amount = amount_a.max(amount_b);
                if max_amount == 0 { 1000 } else {
                    fixed_point::mul_div_down(&env, min_amount, min_amount, max_amount)
                }
            }
        } else {
            // Proportional share formula for subsequent deposits
            // Calculate shares based on the ratio of input amounts to existing reserves
            let shares_a = if reserve_a > 0 {
                fixed_point::mul_div_up(&env, amount_a, total_liquidity, reserve_a)
            } else {
                0
            };
            let shares_b = if reserve_b > 0 {
                fixed_point::mul_div_up(&env, amount_b, total_liquidity, reserve_b)
            } else {
                0
            };
            
            // Return the minimum of the two calculations to ensure proper ratio
            if shares_a == 0 && shares_b == 0 {
                0
            } else if shares_a == 0 {
                shares_b
            } else if shares_b == 0 {
                shares_a
            } else {
                shares_a.min(shares_b)
            }
        }
    }

    // GET RESERVES: Get current token reserves
    pub fn get_reserves(env: &Env) -> (u128, u128) {
        let reserve_a: u128 = env.storage().instance().get(&DataKey::ReserveA)
            .unwrap_or(0u128);
        let reserve_b: u128 = env.storage().instance().get(&DataKey::ReserveB)
            .unwrap_or(0u128);
        (reserve_a, reserve_b)
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

    // FLASH LOAN: Execute flash loan with reentrancy protection
    pub fn flash_loan(
        env: Env,
        borrower: Address,
        token: Address,
        amount: u128,
        callback: Address,
        _callback_data: Bytes
    ) {
        // Check if factory is active
        Self::require_factory_active(&env);
        
        // Check if token is valid
        let token_a: Address = env.storage().instance().get(&DataKey::TokenA)
            .expect("Not initialized");
        let token_b: Address = env.storage().instance().get(&DataKey::TokenB)
            .expect("Not initialized");

        if token != token_a && token != token_b {
            check_and_panic_error(Error::InvalidTokenAddress);
        }

        // Check if pool has enough liquidity
        let (reserve_a, reserve_b) = Self::get_reserves(&env);
        let reserve_for_token = if token == token_a { reserve_a } else { reserve_b };
        if amount > reserve_for_token {
            check_and_panic_error(Error::InsufficientLiquidity);
        }

        // Set flash loan lock
        env.storage().temporary().set(&DataKey::FlashLoanActive, &true);

        // Transfer tokens to borrower
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &borrower, &(amount as i128));

        // Execute callback (in a real implementation, this would call the borrower's contract)
        // For now, we'll simulate the callback with a simple event
        env.events().publish(
            (Symbol::new(&env, "flash_loan_callback"), borrower.clone()),
            (token.clone(), amount, callback)
        );

        // In a real implementation, we would wait for the callback to complete
        // and verify that the loan has been repaid with fees

        // Clear flash loan lock (after successful repayment)
        env.storage().temporary().remove(&DataKey::FlashLoanActive);

        env.events().publish(
            (Symbol::new(&env, "flash_loan_completed"), borrower),
            (token, amount)
        );
    }
}
