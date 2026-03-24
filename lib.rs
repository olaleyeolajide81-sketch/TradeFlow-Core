#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Bytes, BytesN, Env, Map};

mod tests;

#[contracttype]
pub enum DataKey {
    FeeTo, // The address that receives protocol fees
    Pools, // Map of (TokenA, TokenB) -> PoolAddress
    PoolWasmHash, // The Wasm hash of the Pool contract to deploy
    Admin, // The address of the factory admin
}

#[contract]
pub struct FactoryContract;

#[contractimpl]
impl FactoryContract {
    /// Initializes the factory contract. This can only be called once.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment.
    /// * `admin` - The address of the factory admin.
    /// * `fee_to` - The address where protocol fees will be sent.
    /// * `pool_wasm_hash` - The WASM hash of the liquidity pool contract code.
    ///
    /// # Panics
    /// If the contract has already been initialized.
    pub fn initialize_factory(env: Env, admin: Address, fee_to: Address, pool_wasm_hash: BytesN<32>) {
        if env.storage().instance().has(&DataKey::FeeTo) {
            panic!("Factory has already been initialized");
        }
        env.storage().instance().set(&DataKey::FeeTo, &fee_to);
        env.storage().instance().set(&DataKey::Admin, &admin);
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

    /// Sets the recipient of the protocol fees.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment.
    /// * `fee_to` - The new address to receive fees.
    pub fn set_fee_recipient(env: Env, fee_to: Address) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        admin.require_auth();

        let old_fee_to: Address = env.storage().instance().get(&DataKey::FeeTo).unwrap();
        
        env.storage().instance().set(&DataKey::FeeTo, &fee_to);

        // Emit event: ("Admin", "SetFeeTo", old_fee_to, new_fee_to)
        env.events().publish(
            (symbol_short!("Admin"), symbol_short!("SetFeeTo")),
            (old_fee_to, fee_to)
        );
    }

    /// Toggles the status of a specific pool.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment.
    /// * `token_a` - The first token of the pair.
    /// * `token_b` - The second token of the pair.
    /// * `status` - The new status (e.g., 0 = Paused, 1 = Active).
    pub fn toggle_pool_status(env: Env, token_a: Address, token_b: Address, status: u32) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        admin.require_auth();

        // Verify pool exists
        let pool_addr = Self::get_pool(env.clone(), token_a.clone(), token_b.clone());
        if pool_addr.is_none() {
            panic!("Pool does not exist");
        }

        // Emit event: ("Admin", "PoolStatus", token_a, token_b) -> status
        // Note: The factory doesn't store the status locally in this implementation,
        // but emits the event for indexers and transparency.
        env.events().publish(
            (symbol_short!("Admin"), symbol_short!("PoolStatus"), token_a, token_b),
            status
        );
    }
}