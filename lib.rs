#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Bytes, BytesN, Env, Map};

mod tests;

#[contracttype]
pub enum DataKey {
    FeeTo, // The address that receives protocol fees
    Pools, // Map of (TokenA, TokenB) -> PoolAddress
    PoolWasmHash, // The Wasm hash of the Pool contract to deploy
}

#[contract]
pub struct FactoryContract;

#[contractimpl]
impl FactoryContract {
    /// Initializes the factory contract. This can only be called once.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment.
    /// * `fee_to` - The address where protocol fees will be sent.
    /// * `pool_wasm_hash` - The WASM hash of the liquidity pool contract code.
    ///
    /// # Panics
    /// If the contract has already been initialized.
    pub fn initialize_factory(env: Env, fee_to: Address, pool_wasm_hash: BytesN<32>) {
        if env.storage().instance().has(&DataKey::FeeTo) {
            panic!("Factory has already been initialized");
        }
        env.storage().instance().set(&DataKey::FeeTo, &fee_to);
        env.storage().instance().set(&DataKey::PoolWasmHash, &pool_wasm_hash);
    }

    /// Helper function to sort two token addresses, creating a canonical pair
    /// to ensure that get_pool(A, B) == get_pool(B, A).
    fn sort_tokens(token_a: Address, token_b: Address) -> (Address, Address) {
        if token_a < token_b {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        }
    }

    /// Returns the address of the liquidity pool for a given pair of tokens.
    pub fn get_pool(env: Env, token_a: Address, token_b: Address) -> Option<Address> {
        let sorted_tokens = Self::sort_tokens(token_a, token_b);
        let pools: Map<(Address, Address), Address> =
            env.storage().instance().get(&DataKey::Pools).unwrap_or_else(|| Map::new(&env));

        pools.get(sorted_tokens)
    }

    /// Deploys a new liquidity pool for the given token pair.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment.
    /// * `token_a` - The first token address.
    /// * `token_b` - The second token address.
    ///
    /// # Returns
    /// The address of the newly deployed pool.
    pub fn create_pool(env: Env, token_a: Address, token_b: Address) -> Address {
        if token_a == token_b {
            panic!("Tokens must be different");
        }

        let (token_0, token_1) = Self::sort_tokens(token_a, token_b);

        let mut pools: Map<(Address, Address), Address> =
            env.storage().instance().get(&DataKey::Pools).unwrap_or(Map::new(&env));

        if pools.contains_key((token_0.clone(), token_1.clone())) {
            panic!("Pool already exists");
        }

        // Get the contract code hash
        let wasm_hash: BytesN<32> = env.storage().instance().get(&DataKey::PoolWasmHash).expect("Not initialized");

        // Generate a deterministic salt based on the token pair
        // salt = sha256(token_0 + token_1)
        let mut salt_data = Bytes::new(&env);
        salt_data.append(&token_0.to_xdr(&env));
        salt_data.append(&token_1.to_xdr(&env));
        let salt = env.crypto().sha256(&salt_data);

        // Deploy the new pool contract
        let pool_address = env.deployer().with_current_contract(salt).deploy(wasm_hash);

        // Store the new pool
        pools.set((token_0, token_1), pool_address.clone());
        env.storage().instance().set(&DataKey::Pools, &pools);

        pool_address
    }
}