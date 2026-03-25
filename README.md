# TradeFlow-Core: Decentralized Trade Finance on Soroban

![Build Status](https://img.shields.io/badge/build-passing-brightgreen)
![Soroban](https://img.shields.io/badge/soroban-ready-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

**TradeFlow-Core** is the smart contract layer for the TradeFlow protocol. It enables Real-World Asset (RWA) tokenization and decentralized factoring on the Stellar network.

## 🏗 Architecture

The system consists of two decoupled smart contracts:

1.  **`invoice_nft`**: A standard-compliant NFT representing a verified invoice. It holds metadata (IPFS hash, face value, currency, due date).
2.  **`lending_pool`**: An escrow vault where liquidity providers deposit stablecoins (USDC). It accepts `invoice_nft` as collateral to automate loan origination and repayment.

## 💾 Storage Architecture

To optimize for ledger rent costs and scalability, the protocol uses a tiered storage approach:

- **Instance Storage**: Global configuration settings (Admin, Paused state) and counters (Loan IDs).
- **Persistent Storage**: High-cardinality user data (Loans, Invoices, Whitelists).
- **Temporary Storage**: Used for transient data where applicable.

## ⛓️ Live Testnet Deployments

The following contracts are currently active for frontend integration and testing.

| Contract Name | Network | Contract ID |
| :--- | :--- | :--- |
| **Invoice NFT** | Testnet | `CCYU3LOQI34VHVN3ZOSEBHHKL4YK36FMTOEGLRYDUDRGS7JOLLRKCEQM` |
| **Lending Pool** | Testnet | `CDVJMVPLZJKXSJFDY5AWBOUIRN73BKU2SG674MQDH4GRE6BGBPQD33IQ` |

- **Network Passphrase:** `Test SDF Network ; September 2015`
- **RPC Endpoint:** `https://soroban-testnet.stellar.org`

## 🚀 Quick Start

### Prerequisites
- Rust & Cargo (latest stable)
- Stellar CLI v25.1.0+ (`cargo install stellar-cli`)
- WASM Target: `rustup target add wasm32v1-none`

### Build & Test
```bash
# Build all contracts (optimized for WASM)
stellar contract build

# Run the test suite
cargo test