# Swap Execution Flow

This document provides a visual walkthrough of the complete smart contract lifecycle
for a token swap on TradeFlow, from the initial user call to the final on-chain event
emission.

## Sequence Diagram

```mermaid
sequenceDiagram
    autonumber

    actor User
    participant TradeFlowContract
    participant TokenA
    participant TokenB

    User->>TradeFlowContract: swap(user, token_in, amount_in, amount_out_min)

    note over TradeFlowContract: Step 1 — Authorisation
    TradeFlowContract->>TradeFlowContract: require_auth_for_args(token_in, amount_in, amount_out_min)

    note over TradeFlowContract: Step 2 — Check Pause / State
    TradeFlowContract->>TradeFlowContract: load TokenA, TokenB, ReserveA, ReserveB, ProtocolFee
    alt Pool not initialised
        TradeFlowContract-->>User: panic "Not initialized"
    end

    note over TradeFlowContract: Step 3 — Check Balances
    TradeFlowContract->>TradeFlowContract: verify reserve_in > 0
    alt Insufficient liquidity
        TradeFlowContract-->>User: panic "Insufficient liquidity"
    end

    note over TradeFlowContract: Step 4 — Calculate Math
    TradeFlowContract->>TradeFlowContract: amount_in_with_fee = amount_in × (10000 − fee_bps)
    TradeFlowContract->>TradeFlowContract: amount_out = (amount_in_with_fee × reserve_out) ÷ (reserve_in × 10000 + amount_in_with_fee)
    TradeFlowContract->>TradeFlowContract: protocol_fee_amount = amount_in − (amount_in_with_fee ÷ 10000)
    alt amount_out < amount_out_min
        TradeFlowContract-->>User: panic "Insufficient output amount"
    end

    note over TradeFlowContract: Step 5 — TWAP Slippage Guard
    TradeFlowContract->>TradeFlowContract: check_slippage_protection(token_in, amount_in, amount_out)
    TradeFlowContract->>TradeFlowContract: calculate_twap(token_in) → twap_price
    TradeFlowContract->>TradeFlowContract: deviation = |spot_price − twap_price| ÷ twap_price × 10000
    alt deviation > max_deviation (default 1000 bps)
        TradeFlowContract-->>User: panic "Price deviation exceeds TWAP threshold"
    end

    note over TradeFlowContract: Step 6 — Transfer In
    TradeFlowContract->>TokenA: transfer(user → contract, amount_in)
    TokenA-->>TradeFlowContract: OK

    note over TradeFlowContract: Step 7 — Transfer Out
    TradeFlowContract->>TokenB: transfer(contract → user, amount_out)
    TokenB-->>TradeFlowContract: OK

    note over TradeFlowContract: Step 8 — Collect Protocol Fees
    TradeFlowContract->>TradeFlowContract: collect_protocol_fees(token_in, protocol_fee_amount)
    TradeFlowContract->>TradeFlowContract: update FeeAccumulator in storage

    note over TradeFlowContract: Step 9 — Update Reserves
    TradeFlowContract->>TradeFlowContract: ReserveA ← new_reserve_a − fee (if token_in == TokenA)
    TradeFlowContract->>TradeFlowContract: ReserveB ← new_reserve_b − fee (if token_in == TokenB)

    note over TradeFlowContract: Step 10 — Update TWAP Oracle
    TradeFlowContract->>TradeFlowContract: update_price_observation() → store PriceObservation

    note over TradeFlowContract: Step 11 — Emit Event
    TradeFlowContract->>TradeFlowContract: events().publish("swap", (token_in, amount_in, token_out, amount_out, fee))

    TradeFlowContract-->>User: return amount_out
```

## Step-by-Step Breakdown

| # | Step | Actor | Description |
|---|------|--------|-------------|
| 1 | **Authorisation** | TradeFlowContract | Granular auth — user must sign the exact tuple `(token_in, amount_in, amount_out_min)` to prevent front-running. |
| 2 | **Check Pause / State** | TradeFlowContract | Loads pool state (tokens, reserves, fee tier). Panics if the pool has not been initialised. |
| 3 | **Check Balances** | TradeFlowContract | Verifies the input-side reserve is non-zero. Panics with `"Insufficient liquidity"` otherwise. |
| 4 | **Calculate Math** | TradeFlowContract | Applies the constant-product AMM formula `(x · y = k)` with the protocol fee deducted from `amount_in` before the calculation. |
| 5 | **TWAP Slippage Guard** | TradeFlowContract | Compares the spot price implied by this swap against the rolling TWAP. Rejects swaps deviating more than `max_deviation` basis points (default 10 %). |
| 6 | **Transfer In** | TokenA | Pulls `amount_in` of the input token from the user into the contract vault. |
| 7 | **Transfer Out** | TokenB | Pushes `amount_out` of the output token from the contract vault to the user. |
| 8 | **Collect Protocol Fees** | TradeFlowContract | Splits off `protocol_fee_amount` and records it in the `FeeAccumulator` (used for buyback-and-burn). |
| 9 | **Update Reserves** | TradeFlowContract | Writes the new `ReserveA` and `ReserveB` values, net of collected fees, back to contract storage. |
| 10 | **Update TWAP Oracle** | TradeFlowContract | Records a new `PriceObservation` (timestamp, spot prices, cumulative prices) for future slippage calculations. |
| 11 | **Emit Event** | TradeFlowContract | Publishes the `swap` event containing `(token_in, amount_in, token_out, amount_out, fee)` for off-chain indexers and the frontend. |

## Key Formulas

**Constant-product output (fee-adjusted):**

$$
\text{amount\_out} = \frac{(\text{amount\_in} \times (10000 - \text{fee\_bps})) \times \text{reserve\_out}}{(\text{reserve\_in} \times 10000) + (\text{amount\_in} \times (10000 - \text{fee\_bps}))}
$$

**TWAP deviation check:**

$$
\text{deviation\_bps} = \frac{|\text{spot\_price} - \text{twap\_price}|}{\text{twap\_price}} \times 10000
$$

> Swap is rejected if $\text{deviation\_bps} > \text{max\_deviation}$ (default: **1000 bps = 10%**).

## Related Source Files

- Smart contract entry point: [contracts/tradeflow/src/lib.rs](../contracts/tradeflow/src/lib.rs)
- Fixed-point math utilities: [contracts/tradeflow/src/utils/](../contracts/tradeflow/src/utils/)
- Contract tests: [contracts/tradeflow/src/tests.rs](../contracts/tradeflow/src/tests.rs)
- TWAP Oracle documentation: [TWAP_ORACLE_DOCUMENTATION.md](../TWAP_ORACLE_DOCUMENTATION.md)
