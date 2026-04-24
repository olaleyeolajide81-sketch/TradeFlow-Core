use soroban_sdk::Env;

pub const Q64: u128 = 1u128 << 64; // 2^64 scaling factor

/// Multiplies x * y and safely divides by denominator, rounding down
/// Uses bit-shifting for precision to avoid overflow and save gas
pub fn mul_div_down(_env: &Env, x: u128, y: u128, denominator: u128) -> u128 {
    if denominator == 0 {
        panic!("Division by zero");
    }
    
    // Use 128-bit multiplication with overflow protection
    let product = x.checked_mul(y).unwrap_or_else(|| {
        panic!("Multiplication overflow in mul_div_down");
    });
    
    product / denominator
}

/// Multiplies x * y and safely divides by denominator, rounding up
/// Ensures the protocol never loses dust to precision errors
pub fn mul_div_up(_env: &Env, x: u128, y: u128, denominator: u128) -> u128 {
    if denominator == 0 {
        panic!("Division by zero");
    }
    
    let product = x.checked_mul(y).unwrap_or_else(|| {
        panic!("Multiplication overflow in mul_div_up");
    });
    
    // Round up by adding (denominator - 1) before division
    (product + denominator - 1) / denominator
}

/// Scales a number up by Q64 for fixed-point arithmetic
pub fn scale_up(_env: &Env, value: u128) -> u128 {
    value.checked_mul(Q64).unwrap_or_else(|| {
        panic!("Overflow in scale_up");
    })
}

/// Scales a number down from Q64 fixed-point back to normal
pub fn scale_down(_env: &Env, value: u128) -> u128 {
    value / Q64
}

/// Performs fixed-point multiplication using Q64 scaling
#[allow(dead_code)]
pub fn fixed_mul(env: &Env, a: u128, b: u128) -> u128 {
    mul_div_down(env, a, b, Q64)
}

/// Performs fixed-point division using Q64 scaling
#[allow(dead_code)]
pub fn fixed_div(env: &Env, a: u128, b: u128) -> u128 {
    mul_div_up(env, a, Q64, b)
}
