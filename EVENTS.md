# Event Documentation

This document describes the events emitted by the TradeFlow-Core smart contracts, specifically administrative actions for transparency.

## Factory Contract Events

### Admin Action: Set Fee Recipient

Emitted when the admin updates the address that receives protocol fees.

*   **Topics**:
    1.  `"Admin"` (Symbol)
    2.  `"SetFeeTo"` (Symbol)
*   **Data**: `(old_fee_recipient: Address, new_fee_recipient: Address)`

### Admin Action: Toggle Pool Status

Emitted when the admin changes the status of a specific liquidity pool (e.g., pausing or unpausing).

*   **Topics**:
    1.  `"Admin"` (Symbol)
    2.  `"PoolStatus"` (Symbol)
    3.  `token_a` (Address)
    4.  `token_b` (Address)
*   **Data**: `status: u32`
    *   `0`: Paused
    *   `1`: Active
    *   (Other codes may be defined in the future)