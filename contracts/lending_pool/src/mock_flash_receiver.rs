use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, Bytes};

#[contracttype]
pub enum DataKey {
    LendingPool,
    Token,
}

#[contract]
pub struct MockFlashReceiver;

#[contractimpl]
impl MockFlashReceiver {
    pub fn init(env: Env, lending_pool: Address, token: Address) {
        env.storage().instance().set(&DataKey::LendingPool, &lending_pool);
        env.storage().instance().set(&DataKey::Token, &token);
    }

    pub fn execute_operation(env: Env, amount: i128, fee: i128, _params: Bytes) {
        let lending_pool: Address = env.storage().instance().get(&DataKey::LendingPool).unwrap();
        let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token);

        // Approve pool to take amount + fee
        client.approve(&env.current_contract_address(), &lending_pool, &amount + &fee, &env.ledger().timestamp().saturating_add(100));

        // Transfer repayment
        client.transfer(&env.current_contract_address(), &lending_pool, &amount + &fee);
    }
}

