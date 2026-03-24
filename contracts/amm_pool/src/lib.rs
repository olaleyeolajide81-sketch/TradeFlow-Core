#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, symbol_short, Symbol};

mod tests;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolState {
    pub token_a: Address,
    pub token_b: Address,
    // Storing as u8 as explicitly requested
    pub token_a_decimals: u8, 
    pub token_b_decimals: u8,
    pub reserve_a: i128,
    pub reserve_b: i128,
}

#[contracttype]
pub enum DataKey {
    State,
}

#[contract]
pub struct AmmPool;

#[contractimpl]
impl AmmPool {
    /// Initialize the AMM pool with two tokens.
    /// 1. Queries the Stellar network to fetch exact decimal precision via Soroban token interface.
    /// 2. Validates that both values are positive integers <= 18.
    /// 3. Aborts initialization if either token's decimals cannot be determined or are invalid.
    pub fn init(env: Env, token_a: Address, token_b: Address) {
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

        let state = PoolState {
            token_a,
            token_b,
            token_a_decimals: decimals_a as u8,
            token_b_decimals: decimals_b as u8,
            reserve_a: 0,
            reserve_b: 0,
        };

        env.storage().instance().set(&DataKey::State, &state);
    }

    /// Provide liquidity (simplified for testing AMM calculations)
    pub fn provide_liquidity(env: Env, amount_a: i128, amount_b: i128) {
        let mut state: PoolState = env.storage().instance().get(&DataKey::State).expect("Not initialized");
        state.reserve_a = state.reserve_a.saturating_add(amount_a);
        state.reserve_b = state.reserve_b.saturating_add(amount_b);
        env.storage().instance().set(&DataKey::State, &state);
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
}
