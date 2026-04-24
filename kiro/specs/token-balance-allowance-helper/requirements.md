# Requirements Document

## Introduction

The AMM pool contract (`contracts/amm_pool/src/lib.rs`) currently duplicates token balance and allowance verification logic across multiple entry points (`swap`, `provide_liquidity`, and related operations). This duplication inflates the compiled WASM binary size and makes the business logic harder to audit. This feature introduces a single internal helper function, `verify_balance_and_allowance`, that centralises these checks, enforces consistent error handling, and replaces every duplicate block in the contract.

## Glossary

- **AmmPool**: The Soroban smart contract implementing the automated market-maker pool.
- **Helper**: The internal Rust function `verify_balance_and_allowance` introduced by this feature.
- **Token_Client**: The Soroban `token::Client` used to query on-chain token state.
- **Balance**: The on-chain token balance of a given `Address`, returned by `token::Client::balance()`.
- **Allowance**: The spending allowance granted by a user to the contract, returned by `token::Client::allowance()`.
- **Required_Amount**: The minimum token quantity (in the token's native units) that must be both held and approved before a transfer may proceed.
- **Caller**: The `Address` of the user initiating a contract entry-point call.
- **Duplicate_Block**: Any sequence of `token.balance()` / `token.allowance()` calls followed by a conditional `panic!` that appears more than once in the contract source.

## Requirements

### Requirement 1: Helper Function Creation

**User Story:** As a smart-contract auditor, I want all token pre-condition checks consolidated in one place, so that I can verify correctness once rather than hunting for scattered copies.

#### Acceptance Criteria

1. THE AmmPool SHALL expose an internal (non-`pub`) Rust function with the signature `fn verify_balance_and_allowance(env: &Env, token: &Address, user: &Address, required_amount: i128)`.
2. WHEN `verify_balance_and_allowance` is called, THE Helper SHALL invoke `token::Client::new(env, token).balance(user)` to obtain the caller's current balance.
3. WHEN `verify_balance_and_allowance` is called, THE Helper SHALL invoke `token::Client::new(env, token).allowance(user, &env.current_contract_address())` to obtain the caller's current allowance.
4. IF the balance returned is less than `required_amount`, THEN THE Helper SHALL panic with a descriptive error message indicating insufficient balance.
5. IF the allowance returned is less than `required_amount`, THEN THE Helper SHALL panic with a descriptive error message indicating insufficient allowance.
6. THE Helper SHALL NOT be callable from outside the contract (it MUST be `fn`, not `pub fn`).

### Requirement 2: Deduplication of Existing Checks

**User Story:** As a developer, I want the repeated balance and allowance check blocks removed from the contract entry points, so that the compiled WASM binary is smaller and the business logic is easier to follow.

#### Acceptance Criteria

1. THE AmmPool SHALL replace every Duplicate_Block in the contract source with a single call to `verify_balance_and_allowance`.
2. WHEN the refactoring is complete, THE AmmPool source SHALL contain no inline calls to `token::Client::balance` or `token::Client::allowance` outside of `verify_balance_and_allowance`.
3. THE AmmPool SHALL replace at least three Duplicate_Blocks with calls to the Helper.
4. WHEN `verify_balance_and_allowance` is called with a `required_amount` that the user satisfies, THE AmmPool SHALL continue execution of the calling entry point without interruption.

### Requirement 3: Consistent Error Handling

**User Story:** As a contract integrator, I want balance and allowance failures to produce clear, consistent error messages, so that I can diagnose transaction failures quickly.

#### Acceptance Criteria

1. WHEN a balance check fails, THE Helper SHALL panic with a message that includes the string `"insufficient balance"`.
2. WHEN an allowance check fails, THE Helper SHALL panic with a message that includes the string `"insufficient allowance"`.
3. THE Helper SHALL check balance before allowance, so that a user with insufficient balance receives the balance error rather than the allowance error.
4. IF `required_amount` is zero or negative, THEN THE Helper SHALL return without performing any checks.

### Requirement 4: Behavioural Equivalence

**User Story:** As a developer, I want the refactored contract to behave identically to the original for all valid and invalid inputs, so that no regressions are introduced.

#### Acceptance Criteria

1. FOR ALL combinations of token address, user address, and required amount, THE AmmPool SHALL produce the same observable outcome (success or panic) after the refactoring as it did before.
2. WHEN `verify_balance_and_allowance` succeeds, THE AmmPool SHALL not emit any additional events or modify any storage entries.
3. THE AmmPool existing test suite SHALL continue to pass without modification after the refactoring.
