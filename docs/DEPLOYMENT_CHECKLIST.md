# Mainnet Beta Deployment Checklist - TradeFlow Core

This document outlines the mandatory verification steps required before the official Wave 3 launch on Mainnet Beta. The Admin must manually verify and sign off on each item to ensure operational security and contract integrity.

## 1. Administrative Security
- [ ] **Admin Address Verification**: Verify that the `admin_address` passed to the initialization function matches the official hardware wallet address (Cold Storage).
    - Expected Address: `[INSERT_OFFICIAL_ADDRESS_HERE]`
- [ ] **Multi-Sig Configuration**: Ensure that any multi-sig thresholds are correctly set according to the governance protocol.

## 2. Compilation & Profile Settings
- [ ] **Release Profile Validation**: Confirm `Cargo.toml` includes the following optimization and safety flags:
    ```toml
    [profile.release]
    overflow-checks = true
    panic = "abort"
    lto = true
    ```
- [ ] **WASM Optimization**: Ensure the contract is compiled using the latest Soroban optimization toolchain to minimize footprint and gas costs.

## 3. Protocol Parameters
- [ ] **Initial Fee Verification**: Verify that the `protocol_fee` parameters are set to the agreed-upon initial values:
    - Base Fee: 0.3% (30 basis points)
    - Fee Recipient: Official Protocol Treasury Address
- [ ] **Tiered Discount Mockup**: Confirm that the tiered discount scaffolding is present in the logic (Silver: 15%, Gold: 30%) even if not yet active.

## 4. Initialization Guard
- [ ] **Re-initialization Check**: Verify that the `is_initialized` flag is implemented and correctly prevents subsequent calls to `initialize`.

## 5. Mathematical Integrity
- [ ] **Overflow Checks**: Run test suite with `--release` to ensure all mathematical operations (especially swap math) handle overflows safely.
- [ ] **Rounding Logic**: Verify `calculate_amount_in` uses rounding-up logic to protect liquidity providers.

---
*Checked and Verified by:* ____________________
*Date:* ____________________
