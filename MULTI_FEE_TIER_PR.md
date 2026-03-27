# Pull Request: Scaffold Multi-Fee Tier Structure for Factory Pool Creation

## Summary
This PR implements the foundational structure for supporting multiple fee tiers in the Factory contract, enabling capital-efficient routing across different risk profiles. Previously, all pools used a hardcoded 0.3% fee, but now the Factory supports deploying pools with specific, immutable fee tiers.

## Changes Made

### Factory Contract (`lib.rs`)
- **Updated `Pool` struct**: Added `fee_tier: u32` field to store the fee tier in basis points
- **Modified `create_pool` function**: 
  - Added `fee_tier: u32` parameter to function signature
  - Implemented strict validation ensuring fee_tier is only 5, 30, or 100 (representing 0.05%, 0.3%, 1%)
  - Updated pool deployment to pass fee_tier as initialization argument to the pool contract
- **Enhanced documentation**: Updated function comments to reflect new parameter and validation logic

### AMM Pool Contract (`contracts/amm_pool/src/lib.rs`)
- **Updated `PoolState` struct**: Added `fee_tier: u32` field to persist the fee tier
- **Modified `init` function**:
  - Added `fee_tier: u32` parameter to accept fee tier from Factory
  - Implemented fee tier validation (5, 30, or 100 basis points)
  - Store fee_tier in the pool state for future use in swap calculations

### Documentation (`README.md`)
- **Added comprehensive fee tier documentation**:
  - Detailed table showing supported fee tiers (5, 30, 100 basis points)
  - Use cases for each tier (Stable, Standard, Volatile pairs)
  - Code examples demonstrating how to create pools with different fee tiers
  - Clear warnings about supported values only

## Supported Fee Tiers

| Fee Tier | Basis Points | Percentage | Use Case |
|----------|--------------|------------|----------|
| Stable   | 5            | 0.05%      | Stablecoin pairs (USDC/USDT, DAI/USDC) |
| Standard | 30           | 0.30%      | Standard token pairs (ETH/USDC, BTC/USDC) |
| Volatile | 100          | 1.00%      | Highly volatile exotic pairs |

## Benefits

1. **Capital Efficiency**: Different fee structures optimize for various token pair volatilities
2. **Risk-Based Pricing**: Higher fees for volatile pairs compensate for increased risk
3. **Competitive Positioning**: Lower fees for stablecoin pairs attract high-volume trading
4. **Foundation for Routing**: Enables smart routing across pools with optimal fee structures
5. **Immutable Fee Tiers**: Once set, fee tiers cannot be changed, ensuring predictability

## Technical Implementation Details

### Validation Logic
Both Factory and AMM pool contracts implement identical validation:
```rust
if fee_tier != 5 && fee_tier != 30 && fee_tier != 100 {
    panic!("Invalid fee tier. Only 5, 30, or 100 basis points are supported");
}
```

### Deployment Flow
1. Factory validates fee tier before pool deployment
2. Factory deploys pool contract with deterministic salt
3. Factory initializes pool with fee tier parameter
4. Pool validates fee tier and stores in state
5. Pool is ready for operations with immutable fee tier

## Testing

The implementation includes:
- Parameter validation in both contracts
- Revert on invalid fee tiers
- Proper storage and retrieval of fee tier information
- Comprehensive documentation for developers

## Future Enhancements

This scaffolding enables future features such as:
- Smart routing algorithm across different fee tiers
- Fee tier recommendation system based on pair volatility
- Dynamic fee tier selection for optimal capital efficiency
- Analytics on pool performance by fee tier

## Breaking Changes

- **Factory `create_pool` function**: Now requires `fee_tier` parameter
- **AMM Pool `init` function**: Now requires `fee_tier` parameter
- Existing pools will need to be migrated if they need specific fee tiers

## Security Considerations

- Fee tier validation occurs in both Factory and Pool contracts for defense in depth
- Fee tiers are immutable after pool creation
- No admin functions to modify fee tiers post-deployment
- Clear error messages for invalid fee tier attempts

## Issue Resolution

This PR addresses issue #94 by providing the foundational structure for multi-fee tier support while maintaining backward compatibility through proper validation and clear documentation.

## Testing Instructions

1. Deploy Factory contract
2. Attempt to create pools with different fee tiers:
   - `create_pool(token_a, token_b, 5)` - Should succeed
   - `create_pool(token_a, token_b, 30)` - Should succeed  
   - `create_pool(token_a, token_b, 100)` - Should succeed
   - `create_pool(token_a, token_b, 15)` - Should fail with validation error
3. Verify pool state contains correct fee tier
4. Test pool operations work correctly with stored fee tier
