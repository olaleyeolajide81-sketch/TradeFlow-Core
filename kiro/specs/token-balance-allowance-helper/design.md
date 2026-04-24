# Design Document: token-balance-allowance-helper

## Overview

The AMM pool contract currently has no `swap` or `provide_liquidity` implementations that call `token::Client::balance` or `token::Client::allowance` — those entry points are stubs. However, the user story calls for introducing `verify_balance_and_allowance` as a reusable internal helper so that when those entry points are fleshed out (or when any future entry point needs pre-condition checks) the pattern is already in place and consistent.

The design therefore covers:
1. Adding the `verify_balance_and_allowance` helper to `AmmPool`.
2. Wiring it into `provide_liquidity` (which currently accepts raw amounts with no validation) and into a new `swap` entry point stub, giving us the required three call-sites.
3. Ensuring the existing test suite continues to pass.

## Architecture

```
contracts/amm_pool/src/
├── lib.rs          ← AmmPool contract (modified)
│   ├── verify_balance_and_allowance()   ← NEW internal helper
│   ├── provide_liquidity()              ← calls helper (modified)
│   ├── swap()                           ← calls helper (new entry point)
│   └── ... (existing functions unchanged)
└── tests.rs        ← existing tests (unchanged) + new helper tests
```

The helper is a plain `fn` (not `pub fn`) on the `AmmPool` impl block, so it is invisible to external callers and does not appear in the generated contract ABI.

## Components and Interfaces

### `verify_balance_and_allowance`

```rust
fn verify_balance_and_allowance(
    env: &Env,
    token: &Address,
    user: &Address,
    required_amount: i128,
)
```

| Parameter         | Type       | Description                                      |
|-------------------|------------|--------------------------------------------------|
| `env`             | `&Env`     | Soroban environment handle                       |
| `token`           | `&Address` | Address of the SEP-41 token contract             |
| `user`            | `&Address` | Address whose balance and allowance are checked  |
| `required_amount` | `i128`     | Minimum quantity required (native token units)   |

**Behaviour:**
- If `required_amount <= 0`, return immediately (no-op).
- Query `token::Client::new(env, token).balance(user)`.
- If `balance < required_amount` → `panic!("insufficient balance")`.
- Query `token::Client::new(env, token).allowance(user, &env.current_contract_address())`.
- If `allowance < required_amount` → `panic!("insufficient allowance")`.

### `provide_liquidity` (modified)

Adds two calls to `verify_balance_and_allowance` — one for `token_a` and one for `token_b` — before updating reserves. This is call-site 1 and call-site 2.

### `swap` (new entry point)

A minimal `pub fn swap(env: Env, amount_in: i128, is_a_in: bool, user: Address)` that:
1. Calls `verify_balance_and_allowance` for the input token (call-site 3).
2. Delegates to `calculate_amount_out` for the output amount.
3. Does not perform actual token transfers (those require a fuller implementation outside this feature's scope).

## Data Models

No new storage keys or on-chain data structures are introduced. The helper is a pure read-only pre-condition check; it reads token state but writes nothing.

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system — essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*


### Property 1: Insufficient balance causes panic

*For any* token address, user address, and required amount where the user's on-chain balance is strictly less than the required amount, calling `verify_balance_and_allowance` SHALL panic with a message containing `"insufficient balance"`.

**Validates: Requirements 1.4, 3.1**

---

### Property 2: Insufficient allowance causes panic

*For any* token address, user address, and required amount where the user's balance is sufficient but the user's allowance granted to the contract is strictly less than the required amount, calling `verify_balance_and_allowance` SHALL panic with a message containing `"insufficient allowance"`.

**Validates: Requirements 1.5, 3.2**

---

### Property 3: Sufficient balance and allowance allows continuation (includes zero/negative edge case)

*For any* token address, user address, and required amount where the user's balance >= required amount AND the user's allowance >= required amount (or required amount <= 0), calling `verify_balance_and_allowance` SHALL return without panicking.

**Validates: Requirements 2.4, 3.4**

---

### Property 4: Balance is checked before allowance

*For any* token address and user address where both balance and allowance are insufficient, calling `verify_balance_and_allowance` SHALL panic with a message containing `"insufficient balance"` (not `"insufficient allowance"`), confirming balance is evaluated first.

**Validates: Requirements 3.3**

---

### Property 5: No side effects on success

*For any* successful call to `verify_balance_and_allowance`, the contract storage state and emitted events SHALL be identical before and after the call.

**Validates: Requirements 4.2**

---

## Error Handling

| Condition | Panic message |
|-----------|---------------|
| `balance < required_amount` | `"insufficient balance"` |
| `allowance < required_amount` | `"insufficient allowance"` |
| `required_amount <= 0` | *(no panic — early return)* |

All panics propagate as Soroban contract errors and cause the invoking transaction to fail atomically. No partial state changes occur because the helper performs no writes.

## Testing Strategy

### Dual Testing Approach

Both unit tests and property-based tests are required and complementary.

- **Unit tests** cover specific examples, the check-ordering example (Property 4), and integration with `provide_liquidity` / `swap`.
- **Property-based tests** cover Properties 1–3 and 5 across randomly generated inputs.

### Property-Based Testing Library

Use [`proptest`](https://github.com/proptest-rs/proptest) (Rust). Add to `contracts/amm_pool/Cargo.toml` under `[dev-dependencies]`:

```toml
proptest = "1"
```

Each property test MUST run a minimum of 100 iterations (proptest default is 256, which satisfies this).

### Property Test Tags

Each property-based test MUST include a comment in the following format:

```
// Feature: token-balance-allowance-helper, Property <N>: <property_text>
```

### Property Test Mapping

| Property | Test type | Description |
|----------|-----------|-------------|
| P1 | property | For random (balance, required) where balance < required → panic "insufficient balance" |
| P2 | property | For random (balance, allowance, required) where balance >= required but allowance < required → panic "insufficient allowance" |
| P3 | property | For random (balance, allowance, required) where both >= required → no panic |
| P4 | example  | Both balance and allowance insufficient → panic message is "insufficient balance" |
| P5 | property | After successful call, no storage mutation and no events emitted |

### Unit Test Coverage

- `test_helper_panics_on_zero_balance` — balance = 0, required = 1
- `test_helper_panics_on_exact_shortfall` — balance = required - 1
- `test_helper_passes_on_exact_match` — balance = required, allowance = required
- `test_helper_noop_on_zero_required` — required = 0, balance = 0, allowance = 0 → no panic
- `test_helper_noop_on_negative_required` — required = -1 → no panic
- `test_provide_liquidity_calls_helper` — provide_liquidity panics when caller has insufficient balance
- `test_swap_calls_helper` — swap panics when caller has insufficient balance
