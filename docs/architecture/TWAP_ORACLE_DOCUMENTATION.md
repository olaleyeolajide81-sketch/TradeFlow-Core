# TWAP Oracle & Dynamic Slippage Protection

## Overview

The TradeFlow protocol now includes a sophisticated TWAP (Time-Weighted Average Price) oracle that provides dynamic slippage protection against flash crashes and oracle manipulation attacks. This implementation acts as a decentralized circuit breaker, protecting liquidity pools from extreme price volatility.

## Architecture

### Core Components

1. **Price Observation System**
   - Continuously tracks pool prices after each swap
   - Stores cumulative price data for TWAP calculations
   - Maintains timestamp-based price history

2. **TWAP Calculation Engine**
   - Computes time-weighted average prices over configurable windows
   - Uses Q64 fixed-point arithmetic for precision
   - Handles edge cases like insufficient data

3. **Slippage Protection Guard**
   - Validates current spot price against TWAP reference
   - Blocks trades exceeding maximum deviation threshold
   - Configurable protection parameters

## Data Structures

### PriceObservation
```rust
pub struct PriceObservation {
    pub timestamp: u64,              // Observation timestamp
    pub price_a_per_b: u128,         // Current spot price (A in terms of B)
    pub price_b_per_a: u128,         // Current spot price (B in terms of A)
    pub cumulative_price_a: u128,     // Cumulative price for TWAP
    pub cumulative_price_b: u128,     // Cumulative price for TWAP
}
```

### TWAPConfig
```rust
pub struct TWAPConfig {
    pub window_size: u64,      // Time window in seconds (default: 3600 = 1 hour)
    pub max_deviation: u32,    // Max deviation in basis points (default: 1000 = 10%)
    pub enabled: bool,         // Whether protection is active
}
```

## Key Functions

### Oracle Management
- `update_price_observation()`: Records current prices after swaps
- `calculate_twap()`: Computes time-weighted average price
- `check_slippage_protection()`: Validates price deviation

### Configuration
- `set_twap_config()`: Admin-only configuration updates
- `get_twap_config()`: Public configuration viewing

## Protection Mechanism

### How It Works

1. **Price Tracking**: After each successful swap, the oracle records the current pool price
2. **TWAP Calculation**: Computes the average price over the configured time window
3. **Deviation Check**: Before executing new swaps, validates against TWAP reference
4. **Circuit Breaking**: Blocks trades that exceed the maximum allowed deviation

### Default Configuration
- **Window Size**: 1 hour (3600 seconds)
- **Maximum Deviation**: 10% (1000 basis points)
- **Protection**: Enabled by default

## Security Benefits

### Flash Crash Protection
- Prevents sudden, extreme price movements
- Protects against oracle manipulation attacks
- Safeguards liquidity provider funds

### Whale Trade Mitigation
- Blocks trades that would move price significantly
- Maintains market stability
- Reduces sandwich attack vulnerability

### Configurable Safety
- Admin can adjust thresholds based on market conditions
- Can be temporarily disabled for maintenance
- Fine-tuned for different token pairs

## Usage Examples

### Normal Trading
```rust
// Standard swap with TWAP protection
let amount_out = TradeFlow::swap(
    &env,
    user,
    token_a,
    1000,  // amount_in
    950    // amount_out_min
);
// Swap executes if price deviation < 10%
```

### Admin Configuration
```rust
// Update protection parameters
TradeFlow::set_twap_config(
    &env,
    Some(7200),    // 2-hour window
    Some(500),     // 5% max deviation
    Some(true)     // Enable protection
);
```

### Checking Configuration
```rust
let config = TradeFlow::get_twap_config(&env);
println!("Window: {} seconds", config.window_size);
println!("Max deviation: {} bps", config.max_deviation);
println!("Enabled: {}", config.enabled);
```

## Gas Optimization

### Efficient Storage
- Stores only the latest price observation
- Uses cumulative price calculations for efficiency
- Minimal storage overhead per swap

### Computation Optimizations
- Q64 fixed-point arithmetic for precision
- Bit-shifting operations where possible
- Overflow protection with minimal gas cost

## Edge Cases Handled

### Insufficient Liquidity
- Skips price updates when reserves are zero
- Gracefully handles empty pool scenarios

### Insufficient History
- Uses current spot price when TWAP window not established
- Gradually builds price history over time

### Configuration Changes
- Applies new settings immediately
- Maintains existing price observations
- Validates parameter ranges

## Testing Coverage

### Unit Tests
- ✅ TWAP configuration initialization
- ✅ Configuration updates and validation
- ✅ Price observation creation
- ✅ Slippage protection functionality
- ✅ Edge case handling

### Integration Tests
- ✅ Swap flow with protection enabled
- ✅ Configuration changes during operation
- ✅ Disabled protection scenarios

## Monitoring & Events

### Event Emission
- `twap_config_updated`: Configuration changes
- `swap`: Includes protection validation status

### Error Messages
- Clear error messages for protection triggers
- Detailed diagnostics for debugging
- User-friendly failure explanations

## Future Enhancements

### Advanced Features
- Multi-timeframe TWAP calculations
- Volatility-based dynamic thresholds
- Cross-pair price correlation checks

### Performance Optimizations
- Batch price observations
- Compressed historical data
- Gas-efficient storage patterns

## Security Considerations

### Attack Vectors Mitigated
- **Flash Loan Attacks**: Blocked by price deviation checks
- **Oracle Manipulation**: Protected by time-weighted averaging
- **Sandwich Attacks**: Limited by deviation thresholds

### Operational Security
- Admin-only configuration changes
- Graceful degradation on failures
- Comprehensive error handling

---

This TWAP oracle implementation provides enterprise-grade protection for decentralized trading while maintaining the flexibility and efficiency required for modern DeFi applications.
