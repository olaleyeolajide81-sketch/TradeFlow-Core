//! Math utility functions for on-chain number handling.
//!
//! Babylonian square root (integer), tailored for large `u128` values in an EVM-like smart contract setting.

/// Returns the integer square root of `y`.
///
/// This uses the Babylonian method (also known as Heron's method) to compute
/// `floor(sqrt(y))` for `u128` values safely with deterministic convergence.
///
/// Algorithm summary:
/// 1. If `y` is 0 or 1, return `y` immediately.
/// 2. Start with an initial guess `x = y / 2 + 1` (or 1 for small values).
/// 3. Iterate `(x + y / x) / 2` until the result stabilizes.
/// 4. Post-process to ensure the result is not greater than `sqrt(y)`.
///
/// Loop constraints and safety:
/// - Each iteration reduces the difference between guess and true root.
/// - The maximum number of iterations is small (<= 128 for u128), and on-chain this is gas-efficient enough for this domain.
/// - Avoids floating-point arithmetic entirely.
///
/// Example:
/// ```rust
/// use utils::math::sqrt;
/// assert_eq!(sqrt(0), 0);
/// assert_eq!(sqrt(1), 1);
/// assert_eq!(sqrt(15), 3);
/// assert_eq!(sqrt(16), 4);
/// assert_eq!(sqrt(1_000_000_000_000_000_000), 1_000_000_000);
/// ```
pub fn sqrt(y: u128) -> u128 {
    if y < 2 {
        return y;
    }

    // Initial guess: y/2 + 1 gives an upper bound and is safe for y>=2.
    // This is a standard starting point for the Babylonian method.
    let mut x0 = (y >> 1) + 1;
    let mut x1 = y;

    // Iterate until convergence: x1 should stabilize to the floor(sqrt(y)).
    // We use a limited loop to avoid accidental infinite loops in case of unusual behavior,
    // but mathematically this converges quickly for u128.
    for _ in 0..128 {
        // Compute next candidate as (x0 + y / x0) / 2.
        x1 = (x0 + y / x0) >> 1;

        // Stop when we reached fixed point (no further change), or if x1 >= x0.
        // `x1 >= x0` is a safe breaking condition in integer domain.
        if x1 >= x0 {
            break;
        }

        x0 = x1;
    }

    // Final result must be floored (largest integer whose square <= y).
    // x1 may occasionally be one above due to rounding, so adjust.
    while (x1 + 1).saturating_mul(x1 + 1) <= y {
        x1 += 1;
    }
    while x1.saturating_mul(x1) > y {
        x1 -= 1;
    }

    x1
}
