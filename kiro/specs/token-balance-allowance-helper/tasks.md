# Implementation Plan: token-balance-allowance-helper

## Overview

Introduce the `verify_balance_and_allowance` internal helper to `AmmPool`, wire it into `provide_liquidity` and a new `swap` entry point (three call-sites total), and add property-based and unit tests to validate correctness.

## Tasks

- [x] 1. Add `proptest` dev-dependency and set up test scaffolding
  - Add `proptest = "1"` under `[dev-dependencies]` in `contracts/amm_pool/Cargo.toml`
  - Confirm the existing test suite still compiles and passes
  - _Requirements: 4.3_

- [x] 2. Implement `verify_balance_and_allowance` helper
  - [x] 2.1 Write the helper function inside the `AmmPool` impl block in `lib.rs`
    - Signature: `fn verify_balance_and_allowance(env: &Env, token: &Address, user: &Address, required_amount: i128)`
    - Early-return if `required_amount <= 0`
    - Query `token::Client::new(env, token).balance(user)`; panic with `"insufficient balance"` if below threshold
    - Query `token::Client::new(env, token).allowance(user, &env.current_contract_address())`; panic with `"insufficient allowance"` if below threshold
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 3.1, 3.2, 3.3, 3.4_

  - [x]* 2.2 Write unit tests for the helper (in `tests.rs`)
    - `test_helper_panics_on_insufficient_balance` — balance < required → panic "insufficient balance"
    - `test_helper_panics_on_insufficient_allowance` — balance ok, allowance < required → panic "insufficient allowance"
    - `test_helper_passes_on_exact_match` — balance == required, allowance == required → no panic
    - `test_helper_noop_on_zero_required` — required = 0 → no panic even with zero balance/allowance
    - `test_helper_noop_on_negative_required` — required = -1 → no panic
    - `test_balance_checked_before_allowance` — both insufficient → panic message is "insufficient balance"
    - _Requirements: 1.4, 1.5, 3.1, 3.2, 3.3, 3.4_

  - [x]* 2.3 Write property test: Property 1 — insufficient balance causes panic
    - `// Feature: token-balance-allowance-helper, Property 1: insufficient balance causes panic`
    - Use proptest to generate random `(balance, required)` where `balance < required`
    - Assert the call panics with a message containing `"insufficient balance"`
    - _Requirements: 1.4, 3.1_

  - [x]* 2.4 Write property test: Property 2 — insufficient allowance causes panic
    - `// Feature: token-balance-allowance-helper, Property 2: insufficient allowance causes panic`
    - Generate random `(balance, allowance, required)` where `balance >= required` but `allowance < required`
    - Assert the call panics with a message containing `"insufficient allowance"`
    - _Requirements: 1.5, 3.2_

  - [x]* 2.5 Write property test: Property 3 — sufficient inputs allow continuation
    - `// Feature: token-balance-allowance-helper, Property 3: sufficient balance and allowance allows continuation`
    - Generate random `(balance, allowance, required)` where `balance >= required` and `allowance >= required`
    - Assert the call does not panic
    - Also cover `required <= 0` edge case within the same generator
    - _Requirements: 2.4, 3.4_

- [x] 3. Checkpoint — ensure all tests pass
  - Run `cargo test -p amm_pool` and confirm zero failures before proceeding.

- [x] 4. Wire helper into `provide_liquidity` (call-sites 1 and 2)
  - [x] 4.1 Update `provide_liquidity` in `lib.rs` to accept a `user: Address` parameter
    - Add `user.require_auth()` at the top
    - Call `Self::verify_balance_and_allowance(&env, &state.token_a, &user, amount_a)` before updating reserves
    - Call `Self::verify_balance_and_allowance(&env, &state.token_b, &user, amount_b)` before updating reserves
    - _Requirements: 2.1, 2.3, 2.4_

  - [x]* 4.2 Write unit test: `test_provide_liquidity_calls_helper`
    - Set up a mock token with balance < amount_a for the user
    - Assert `provide_liquidity` panics with `"insufficient balance"`
    - _Requirements: 2.1, 2.3_

- [x] 5. Add `swap` entry point (call-site 3)
  - [x] 5.1 Implement `pub fn swap(env: Env, user: Address, amount_in: i128, is_a_in: bool) -> i128` in `lib.rs`
    - Load pool state; panic if not initialised
    - Determine input token from `is_a_in`
    - Call `Self::verify_balance_and_allowance(&env, &input_token, &user, amount_in)` (call-site 3)
    - Delegate to `Self::calculate_amount_out` and return the result
    - _Requirements: 2.1, 2.3, 2.4_

  - [x]* 5.2 Write unit test: `test_swap_calls_helper`
    - Set up a mock token with balance < amount_in for the user
    - Assert `swap` panics with `"insufficient balance"`
    - _Requirements: 2.1, 2.3_

  - [x]* 5.3 Write property test: Property 5 — no side effects on success
    - `// Feature: token-balance-allowance-helper, Property 5: no side effects on success`
    - For random valid inputs, snapshot storage before calling the helper, call it, snapshot after
    - Assert storage and events are unchanged
    - _Requirements: 4.2_

- [x] 6. Final checkpoint — ensure all tests pass
  - Run `cargo test -p amm_pool` and confirm zero failures.
  - Verify no `token::Client::balance` or `token::Client::allowance` calls exist outside `verify_balance_and_allowance` in `lib.rs`.

## Notes

- Tasks marked with `*` are optional and can be skipped for a faster MVP.
- Each task references specific requirements for traceability.
- Property tests use `proptest` with default 256 iterations (exceeds the 100-iteration minimum).
- The `swap` entry point in task 5 is intentionally minimal — it does not perform actual token transfers, which are out of scope for this feature.
