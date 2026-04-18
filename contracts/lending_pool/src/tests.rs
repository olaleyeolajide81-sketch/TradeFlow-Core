#[cfg(test)]
mod tests {
    use soroban_sdk::{Address, Env, token, testutils::{Ledger, Address as _}};
    use crate::LendingPoolClient;

    #[test]
    fn test_initialization() {
        let env = Env::default();
        let contract_id = env.register(crate::LendingPool, ());
        let client = LendingPoolClient::new(&env, &contract_id);
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_address = Address::generate(&env);

        client.init(&admin, &token_address);

        assert!(!client.is_paused());
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #9)")] // Unauthorized = 9
    fn test_double_initialization() {
        let env = Env::default();
        let contract_id = env.register(crate::LendingPool, ());
        let client = LendingPoolClient::new(&env, &contract_id);
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_address = Address::generate(&env);

        client.init(&admin, &token_address);
        client.init(&admin, &token_address);
    }

    #[test]
    fn test_pause_functionality() {
        let env = Env::default();
        let contract_id = env.register(crate::LendingPool, ());
        let client = LendingPoolClient::new(&env, &contract_id);
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_address = Address::generate(&env);
        client.init(&admin, &token_address);

        // Test pausing
        client.set_paused(&true);
        assert!(client.is_paused());

        // Test unpausing
        client.set_paused(&false);
        assert!(!client.is_paused());
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #2)")] // ContractPaused = 2
    fn test_deposit_when_paused() {
        let env = Env::default();
        let contract_id = env.register(crate::LendingPool, ());
        let client = LendingPoolClient::new(&env, &contract_id);
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_address = Address::generate(&env);
        client.init(&admin, &token_address);

        client.set_paused(&true);

        let user = Address::generate(&env);
        client.deposit(&user, &1000);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #11)")] // PoolIsPaused = 11
    fn test_borrow_when_paused() {
        let env = Env::default();
        let contract_id = env.register(crate::LendingPool, ());
        let client = LendingPoolClient::new(&env, &contract_id);
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_address = Address::generate(&env);
        client.init(&admin, &token_address);

        client.set_paused(&true);

        let borrower = Address::generate(&env);
        client.borrow(&borrower, &1000);
    }

    #[test]
    fn test_create_loan() {
        let env = Env::default();
        let contract_id = env.register(crate::LendingPool, ());
        let client = LendingPoolClient::new(&env, &contract_id);
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_address = Address::generate(&env);
        client.init(&admin, &token_address);

        let borrower = Address::generate(&env);
        let due_date = env.ledger().timestamp() + 86400;
        let loan_id = client.create_loan(&borrower, &1, &1000, &due_date);

        let loan = client.get_loan(&loan_id).unwrap();
        assert_eq!(loan.borrower, borrower);
        assert_eq!(loan.principal, 1000);
        assert_eq!(loan.invoice_id, 1);
        assert!(!loan.is_repaid);
        assert!(!loan.is_defaulted);
    }

    #[test]
    fn test_repay_loan() {
        let env = Env::default();
        let contract_id = env.register(crate::LendingPool, ());
        let client = LendingPoolClient::new(&env, &contract_id);
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_address = env.register_stellar_asset_contract_v2(admin.clone()).address();
        client.init(&admin, &token_address);

        let borrower = Address::generate(&env);
        let due_date = env.ledger().timestamp() + 86400;
        let loan_id = client.create_loan(&borrower, &1, &1000, &due_date);

        let token_client = token::StellarAssetClient::new(&env, &token_address);
        token_client.mint(&borrower, &2000);

        client.repay_loan(&loan_id);

        let loan = client.get_loan(&loan_id).unwrap();
        assert!(loan.is_repaid);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #5)")] // LoanAlreadyRepaid = 5
    fn test_repay_already_repaid_loan() {
        let env = Env::default();
        let contract_id = env.register(crate::LendingPool, ());
        let client = LendingPoolClient::new(&env, &contract_id);
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_address = env.register_stellar_asset_contract_v2(admin.clone()).address();
        client.init(&admin, &token_address);

        let borrower = Address::generate(&env);
        let due_date = env.ledger().timestamp() + 86400;
        let loan_id = client.create_loan(&borrower, &1, &1000, &due_date);

        let token_client = token::StellarAssetClient::new(&env, &token_address);
        token_client.mint(&borrower, &2000);

        client.repay_loan(&loan_id);
        client.repay_loan(&loan_id);
    }

    #[test]
    fn test_liquidate_loan() {
        let env = Env::default();
        let contract_id = env.register(crate::LendingPool, ());
        let client = LendingPoolClient::new(&env, &contract_id);
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_address = env.register_stellar_asset_contract_v2(admin.clone()).address();
        client.init(&admin, &token_address);

        let borrower = Address::generate(&env);
        // We have to mock a loan with a past date manually or adjust the ledger
        // For the sake of this test, we assume create_loan logic is tested elsewhere
        let loan_id = client.create_loan(&borrower, &1, &1000, &(env.ledger().timestamp() + 1));
        
        // Fast forward time
        env.ledger().set_timestamp(env.ledger().timestamp() + 86401);

        let liquidator = Address::generate(&env);
        let token_client = token::StellarAssetClient::new(&env, &token_address);
        token_client.mint(&liquidator, &2000);

        client.liquidate(&liquidator, &loan_id);

        let loan = client.get_loan(&loan_id).unwrap();
        assert!(loan.is_defaulted);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #10)")]
    fn test_math_overflow_in_create_loan() {
        let env = Env::default();
        let contract_id = env.register(crate::LendingPool, ());
        let client = LendingPoolClient::new(&env, &contract_id);
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_address = Address::generate(&env);
        client.init(&admin, &token_address);

        let borrower = Address::generate(&env);
        let due_date = env.ledger().timestamp() + 86400;
        
        // This will cause an overflow in calculate_interest when multiplied by APY
        client.create_loan(&borrower, &1, &i128::MAX, &due_date);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #8)")] // CannotLiquidateHealthyLoan = 8
    fn test_liquidate_healthy_loan() {
        let env = Env::default();
        let contract_id = env.register(crate::LendingPool, ());
        let client = LendingPoolClient::new(&env, &contract_id);
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_address = Address::generate(&env);
        client.init(&admin, &token_address);

        let borrower = Address::generate(&env);
        let future_date = env.ledger().timestamp() + 86400; // Future due date
        let loan_id = client.create_loan(&borrower, &1, &1000, &future_date);

        let liquidator = Address::generate(&env);
        client.liquidate(&liquidator, &loan_id);
    }

    #[test]
    fn test_interest_calculation() {
        let env = Env::default();
        let contract_id = env.register(crate::LendingPool, ());
        let client = LendingPoolClient::new(&env, &contract_id);
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_address = Address::generate(&env);
        client.init(&admin, &token_address);

        let borrower = Address::generate(&env);
        let one_year_later = env.ledger().timestamp() + 31_536_000; // 1 year
        let loan_id = client.create_loan(&borrower, &1, &1000, &one_year_later);

        let loan = client.get_loan(&loan_id).unwrap();
        // 5% of 1000 = 50 interest for 1 year
        assert_eq!(loan.interest, 50);
    }

    #[test]
    fn test_calculate_flash_fee() {
        let env = Env::default();
        let contract_id = env.register(crate::LendingPool, ());
        let client = LendingPoolClient::new(&env, &contract_id);

        // 10000 * 8 / 10000 = 8
        assert_eq!(client.calculate_flash_fee(&10000), 8);
        
        // 5000 * 8 / 10000 = 4
        assert_eq!(client.calculate_flash_fee(&5000), 4);
        
        // Zero amount
        assert_eq!(client.calculate_flash_fee(&0), 0);
        
        // Large amount: 1_000_000 * 8 / 10000 = 800
        assert_eq!(client.calculate_flash_fee(&1_000_000), 800);
        
        // Max limit consideration: using i128, large amounts won't overflow
        let large_amount: i128 = 1_000_000_000_000_000;
        assert_eq!(client.calculate_flash_fee(&large_amount), 800_000_000_000);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #10)")] // MathOverflow = 10
    fn test_calculate_flash_fee_negative() {
        let env = Env::default();
        let contract_id = env.register(crate::LendingPool, ());
        let client = LendingPoolClient::new(&env, &contract_id);

        client.calculate_flash_fee(&-100);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #10)")] // MathOverflow = 10
    fn test_flash_loan_zero_amount() {
        let env = Env::default();
        let contract_id = env.register(crate::LendingPool, ());
        let client = LendingPoolClient::new(&env, &contract_id);
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_address = Address::generate(&env);
        client.init(&admin, &token_address);

        let receiver = Address::generate(&env);
        let params = soroban_sdk::Bytes::new(&env);
        
        client.flash_loan(&receiver, &0, &params);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #3)")] // InsufficientLiquidity = 3
    fn test_flash_loan_insufficient_liquidity() {
        let env = Env::default();
        let contract_id = env.register(crate::LendingPool, ());
        let client = LendingPoolClient::new(&env, &contract_id);
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_address = env.register_stellar_asset_contract_v2(admin.clone()).address();
        client.init(&admin, &token_address);

        let receiver = Address::generate(&env);
        client.flash_loan(&receiver, &1000000, &soroban_sdk::Bytes::new(&env));
    }

    // Note: We cannot easily test swap success/fail completely here without a real token contract 
    // mock because client.balance() will panic or return 0, leading to EmptyPool.
    // We can at least test the EmptyPool error when balance is 0.
    #[test]
    #[should_panic(expected = "Error(Contract, #12)")] // EmptyPool = 12
    fn test_swap_empty_pool() {
        let env = Env::default();
        let contract_id = env.register(crate::LendingPool, ());
        let client = LendingPoolClient::new(&env, &contract_id);
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_address = env.register_stellar_asset_contract_v2(admin.clone()).address();
        client.init(&admin, &token_address);

        let user = Address::generate(&env);
        // Will fail with EmptyPool because the token mock has 0 balance
        client.swap(&user, &100);
    }
}
