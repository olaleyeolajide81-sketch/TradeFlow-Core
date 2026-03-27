#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, symbol_short, Symbol};

mod tests;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolState {
    pub token_a: Address,
    pub token_b: Address,
    // Storing as u32 to match token interface
    pub token_a_decimals: u32, 
    pub token_b_decimals: u32,
    pub reserve_a: i128,
    pub reserve_b: i128,
    pub fee_tier: u32, // Fee tier in basis points (5, 30, or 100)
    pub is_deprecated: bool,
    pub _status: u32, // 0 = unlocked, 1 = locked (reentrancy protection)
    // TWAP Oracle state variables
    pub price_0_cumulative_last: u128, // Cumulative price for token_0
    pub price_1_cumulative_last: u128, // Cumulative price for token_1
    pub block_timestamp_last: u32,     // Last update timestamp
}

#[contracttype]
pub enum DataKey {
    State,
    Admin,
}

#[contract]
pub struct AmmPool;

#[contractimpl]
impl AmmPool {
    /// Initialize the AMM pool with two tokens, admin, and fee tier.
    /// 1. Queries the Stellar network to fetch exact decimal precision via Soroban token interface.
    /// 2. Validates that both values are positive integers <= 18.
    /// 3. Validates fee tier is one of the supported values (5, 30, or 100 basis points).
    /// 4. Aborts initialization if validation fails.
    pub fn init(env: Env, admin: Address, token_a: Address, token_b: Address, fee_tier: u32) {
        if env.storage().instance().has(&DataKey::State) {
            panic!("Already initialized");
        }

        let client_a = token::Client::new(&env, &token_a);
        let client_b = token::Client::new(&env, &token_b);

        let decimals_a = client_a.decimals();
        let decimals_b = client_b.decimals();

        if decimals_a == 0 || decimals_a > 18 {
            panic!("Invalid decimals for token_a");
        }
        if decimals_b == 0 || decimals_b > 18 {
            panic!("Invalid decimals for token_b");
        }

        // Validate fee tier
        if fee_tier != 5 && fee_tier != 30 && fee_tier != 100 {
            panic!("Invalid fee tier. Only 5, 30, or 100 basis points are supported");
        }

        let state = PoolState {
            token_a,
            token_b,
            token_a_decimals: decimals_a,
            token_b_decimals: decimals_b,
            reserve_a: 0,
            reserve_b: 0,
            fee_tier,
            is_deprecated: false,
            _status: 0, // Start unlocked
            // Initialize TWAP oracle state
            price_0_cumulative_last: 0,
            price_1_cumulative_last: 0,
            block_timestamp_last: 0,
        };

        env.storage().instance().set(&DataKey::State, &state);
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Provide liquidity (simplified for testing AMM calculations)
    pub fn provide_liquidity(env: Env, amount_a: i128, amount_b: i128) {
        let mut state: PoolState = env.storage().instance().get(&DataKey::State).expect("Not initialized");
        state.reserve_a = state.reserve_a.saturating_add(amount_a);
        state.reserve_b = state.reserve_b.saturating_add(amount_b);
        env.storage().instance().set(&DataKey::State, &state);
    }

    /// Helper function to check admin authorization
    fn require_admin(env: &Env) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        admin.require_auth();
    }

    /// Emergency eject liquidity - Admin only function to forcefully withdraw all liquidity
    /// from a deprecated pool and return underlying tokens to LPs based on snapshot balances.
    /// Requirements:
    /// - Must be called by Admin
    /// - Pool must be deprecated (is_deprecated = true)
    /// - Pool must not be locked by reentrancy (_status = 0)
    pub fn emergency_eject_liquidity(env: Env) {
        // Check admin authorization
        Self::require_admin(&env);

        // Get current pool state
        let mut state: PoolState = env.storage().instance().get(&DataKey::State).expect("Not initialized");

        // Verify pool is deprecated
        if !state.is_deprecated {
            panic!("Pool is not deprecated - emergency eject not allowed");
        }

        // Verify pool is not locked by reentrancy
        if state._status != 0 {
            panic!("Pool is locked - reentrancy protection active");
        }

        // Set reentrancy lock
        state._status = 1;
        env.storage().instance().set(&DataKey::State, &state);

        // Emit massive ProtocolEmergencyEject event to alert all indexers
        env.events().publish(
            (symbol_short!("EmergEjct"), symbol_short!("CRITICAL")), 
            (env.current_contract_address(), state.token_a.clone(), state.token_b.clone(), state.reserve_a, state.reserve_b)
        );

        // TODO: This is where the complex iteration over LP token holders would happen
        // For this issue, we're just scaffolding the state requirements and modifiers
        // The actual implementation would:
        // 1. Iterate through all LP token holders
        // 2. Calculate each LP's share based on their snapshot balances
        // 3. Transfer proportional underlying tokens to each LP
        // 4. Burn LP tokens
        // 5. Reset pool reserves to zero

        // Reset pool state after eject
        state.reserve_a = 0;
        state.reserve_b = 0;
        state._status = 0; // Unlock reentrancy protection
        
        env.storage().instance().set(&DataKey::State, &state);

        // Emit completion event
        env.events().publish(
            (symbol_short!("EmergEjct"), symbol_short!("COMPLETED")), 
            env.current_contract_address()
        );
    }

    /// Calculate the output amount for a given input amount.
    /// 
    /// Scaling formulas:
    /// - scaled = raw * 10^(18 - token_decimals)
    /// - output_native = output_scaled / 10^(18 - target_decimals)
    ///
    /// Uses saturating arithmetic to prevent overflows and rounding half-up for output.
    pub fn calculate_amount_out(env: Env, amount_in: i128, is_a_in: bool) -> i128 {
        let state: PoolState = env.storage().instance().get(&DataKey::State).expect("Not initialized");

        let (decimals_in, decimals_out, reserve_in, reserve_out) = if is_a_in {
            (state.token_a_decimals, state.token_b_decimals, state.reserve_a, state.reserve_b)
        } else {
            (state.token_b_decimals, state.token_a_decimals, state.reserve_b, state.reserve_a)
        };

        if amount_in <= 0 {
            return 0;
        }
        if reserve_in <= 0 || reserve_out <= 0 {
            return 0; // Pool is empty
        }

        // Scale inputs to 18-decimal precision using saturating arithmetic
        let scale_in = 10i128.pow((18 - decimals_in) as u32);
        let amount_in_scaled = amount_in.saturating_mul(scale_in);
        let reserve_in_scaled = reserve_in.saturating_mul(scale_in);

        let scale_out = 10i128.pow((18 - decimals_out) as u32);
        let reserve_out_scaled = reserve_out.saturating_mul(scale_out);

        // Constant-product calculation (x * y = k)
        // amount_out_scaled = (reserve_out_scaled * amount_in_scaled) / (reserve_in_scaled + amount_in_scaled)
        let numerator = reserve_out_scaled.saturating_mul(amount_in_scaled);
        let denominator = reserve_in_scaled.saturating_add(amount_in_scaled);
        
        if denominator == 0 {
            return 0;
        }
        
        let output_scaled = numerator / denominator;

        // Scale back to target token's native decimals with round half-up
        // output_native = (output_scaled + (scale_out / 2)) / scale_out
        let half_scale_out = scale_out / 2;
        let output_native = output_scaled.saturating_add(half_scale_out) / scale_out;

        // Return zero if the scaled output is below the target token's smallest unit
        if output_native == 0 {
            return 0;
        }

        // Calculate precision loss
        let output_scaled_from_native = output_native.saturating_mul(scale_out);
        let loss = output_scaled.abs_diff(output_scaled_from_native);
        
        // Emit debug event if precision loss exceeds 0.01% (i.e., loss * 10000 > output_scaled)
        if loss.saturating_mul(10000) > output_scaled.unsigned_abs() {
            env.events().publish((symbol_short!("warn"), symbol_short!("prec_loss")), loss as i128);
        }

        output_native
    }

    /// Calculate the time elapsed since the last oracle update
    /// 
    /// # Returns
    /// * `u32` - Time elapsed in seconds since last update
    fn calculate_time_elapsed(env: &Env, last_timestamp: u32) -> u32 {
        let current_timestamp = env.ledger().timestamp();
        
        // Handle edge case where current timestamp is less than last timestamp
        if current_timestamp <= last_timestamp {
            return 0;
        }
        
        current_timestamp - last_timestamp
    }

    /// Update the TWAP oracle with current prices
    /// 
    /// This function should be called after any swap to update the cumulative price tracking.
    /// It calculates the time elapsed since the last update and adds the price * time product
    /// to the cumulative trackers.
    /// 
    /// # Arguments
    /// * `env` - The Soroban environment
    pub fn update_twap_oracle(env: Env) {
        let mut state: PoolState = env.storage().instance().get(&DataKey::State).expect("Not initialized");
        
        // Skip update if pool is empty (no reserves)
        if state.reserve_a <= 0 || state.reserve_b <= 0 {
            return;
        }
        
        // Calculate time elapsed since last update
        let time_elapsed = Self::calculate_time_elapsed(&env, state.block_timestamp_last);
        
        // Skip update if no time has passed
        if time_elapsed == 0 {
            return;
        }
        
        // Calculate current prices (scaled to maintain precision)
        // Price = reserve_other / reserve_this
        // We use u128 for cumulative prices to prevent overflow over time
        
        // Price of token_a in terms of token_b (reserve_b / reserve_a)
        let price_a_scaled = if state.reserve_a > 0 {
            // Scale price to maintain precision (multiply by 1e18)
            (state.reserve_b as u128).saturating_mul(1_000_000_000_000_000_000) / (state.reserve_a as u128)
        } else {
            0
        };
        
        // Price of token_b in terms of token_a (reserve_a / reserve_b)
        let price_b_scaled = if state.reserve_b > 0 {
            // Scale price to maintain precision (multiply by 1e18)
            (state.reserve_a as u128).saturating_mul(1_000_000_000_000_000_000) / (state.reserve_b as u128)
        } else {
            0
        };
        
        // Update cumulative prices: cumulative += price * time_elapsed
        state.price_0_cumulative_last = state.price_0_cumulative_last.saturating_add(
            price_a_scaled.saturating_mul(time_elapsed as u128)
        );
        state.price_1_cumulative_last = state.price_1_cumulative_last.saturating_add(
            price_b_scaled.saturating_mul(time_elapsed as u128)
        );
        
        // Update the last timestamp to current time
        state.block_timestamp_last = env.ledger().timestamp();
        
        // Save the updated state
        env.storage().instance().set(&DataKey::State, &state);
        
        // Emit debug event for monitoring
        env.events().publish(
            (symbol_short!("TWAP"), symbol_short!("Update")),
            (time_elapsed, price_a_scaled, price_b_scaled)
        );
    }

    /// Get the current TWAP oracle state
    /// 
    /// # Returns
    /// * `(u128, u128, u32)` - (price_0_cumulative_last, price_1_cumulative_last, block_timestamp_last)
    pub fn get_twap_oracle_state(env: Env) -> (u128, u128, u32) {
        let state: PoolState = env.storage().instance().get(&DataKey::State).expect("Not initialized");
        (
            state.price_0_cumulative_last,
            state.price_1_cumulative_last,
            state.block_timestamp_last
        )
    }
}
