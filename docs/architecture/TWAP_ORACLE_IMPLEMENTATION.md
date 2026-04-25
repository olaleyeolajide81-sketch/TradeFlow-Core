# TWAP Oracle Implementation Documentation

## Overview

This document describes the Time-Weighted Average Price (TWAP) oracle implementation for the TradeFlow Core AMM pool. The TWAP oracle provides on-chain price feeds that are resistant to flash loan manipulation by tracking cumulative prices over time.

## Purpose

Flash loan attackers can manipulate pool prices within a single ledger by executing large trades and reversing them in the same transaction. A TWAP oracle prevents this by:

1. Recording cumulative prices over time periods
2. Making manipulation economically infeasible (requires sustained price pressure)
3. Providing reliable price feeds for other DeFi protocols

## Architecture

### State Variables

The following state variables have been added to the `PoolState` struct:

```rust
pub price_0_cumulative_last: u128, // Cumulative price for token_0
pub price_1_cumulative_last: u128, // Cumulative price for token_1
pub block_timestamp_last: u32,     // Last update timestamp
```

- `price_0_cumulative_last`: Cumulative price of token_a in terms of token_b
- `price_1_cumulative_last`: Cumulative price of token_b in terms of token_a  
- `block_timestamp_last`: Timestamp of the last oracle update

### Core Functions

#### `calculate_time_elapsed(env: &Env, last_timestamp: u32) -> u32`

Calculates the time elapsed since the last oracle update.

**Returns:** Time elapsed in seconds

**Edge Cases:**
- Returns 0 if current timestamp ≤ last timestamp
- Prevents negative time calculations

#### `update_twap_oracle(env: Env)`

Updates the TWAP oracle with current pool prices. This function should be called after any swap operation.

**Logic:**
1. Skip update if pool has no reserves
2. Calculate time elapsed since last update
3. Skip update if no time has passed
4. Calculate current prices (scaled to 18 decimals for precision)
5. Update cumulative prices: `cumulative += price * time_elapsed`
6. Update last timestamp to current time
7. Emit debug events for monitoring

**Price Calculation:**
```rust
// Price of token_a in terms of token_b
price_a = (reserve_b * 1e18) / reserve_a

// Price of token_b in terms of token_a  
price_b = (reserve_a * 1e18) / reserve_b
```

#### `get_twap_oracle_state(env: Env) -> (u128, u128, u32)`

Returns the current TWAP oracle state.

**Returns:** Tuple of (price_0_cumulative_last, price_1_cumulative_last, block_timestamp_last)

## Mathematical Foundation

### Cumulative Price Tracking

The TWAP oracle tracks cumulative prices using the formula:

```
cumulative_price += current_price * time_elapsed
```

Where:
- `current_price` is the instantaneous pool price (scaled to 18 decimals)
- `time_elapsed` is the time since the last update (in seconds)

### Time-Weighted Average Price Calculation

To calculate the TWAP over a time period `[t1, t2]`:

```
twap = (cumulative_price_t2 - cumulative_price_t1) / (t2 - t1)
```

This provides the average price over the period, weighted by time.

## Security Considerations

### Flash Loan Resistance

The TWAP oracle is resistant to flash loan manipulation because:

1. **Time Weighting:** Price manipulation requires sustained pressure over multiple ledgers
2. **Economic Cost:** Attackers would need to maintain large positions for extended periods
3. **Cumulative Nature:** Single-transaction price spikes have minimal impact on the cumulative average

### Overflow Protection

- Uses `u128` for cumulative prices to prevent overflow over long time periods
- Uses saturating arithmetic to prevent panic conditions
- Prices are scaled to 18 decimals to maintain precision

### Edge Case Handling

- Skips updates when pool has no reserves
- Returns 0 time elapsed for timestamp edge cases
- Prevents division by zero in price calculations

## Integration Points

### When to Call `update_twap_oracle`

The `update_twap_oracle` function should be called after:

1. **Swap operations:** After any token swap completes
2. **Liquidity operations:** After significant liquidity changes
3. **Periodic updates:** On a regular schedule (e.g., every ledger)

### Price Consumption

Other contracts can consume TWAP prices by:

1. Calling `get_twap_oracle_state` to get current cumulative values
2. Storing snapshots at different time points
3. Calculating TWAP over desired time periods

## Gas Optimization

The implementation is designed for gas efficiency:

- Minimal storage updates (only 3 state variables)
- Efficient arithmetic operations
- Conditional updates (skips when no time elapsed)
- Batchable with other pool operations

## Future Enhancements

Potential improvements for future versions:

1. **Multiple TWAP periods:** Track different time windows simultaneously
2. **Price confidence intervals:** Add volatility metrics
3. **External price feeds:** Integration with off-chain oracles
4. **Update throttling:** Rate limiting to prevent spam

## Testing Considerations

When testing the TWAP oracle:

1. **Time manipulation:** Test with different time intervals
2. **Price volatility:** Test with extreme price movements
3. **Edge cases:** Empty pools, zero reserves, timestamp edge cases
4. **Long-running:** Test cumulative behavior over extended periods
5. **Integration:** Test with actual swap operations

## Usage Example

```rust
// After a swap operation
amm_pool::update_twap_oracle(env);

// Later, to calculate TWAP over a period
let (cumulative_now, _, timestamp_now) = amm_pool::get_twap_oracle_state(env);
let (cumulative_before, _, timestamp_before) = stored_snapshot;

let time_elapsed = timestamp_now - timestamp_before;
let twap_price = (cumulative_now - cumulative_before) / time_elapsed;
```

This implementation provides a solid foundation for reliable on-chain price feeds that can be used throughout the TradeFlow ecosystem.
