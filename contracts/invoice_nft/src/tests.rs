#[cfg(test)]
mod tests {
    use soroban_sdk::{Address, Env, BytesN, testutils::Address as _};
    use crate::InvoiceContractClient;

    #[test]
    fn test_mint_invoice_success() {
        let env = Env::default();
        let contract_id = env.register(crate::InvoiceContract, ());
        let client = InvoiceContractClient::new(&env, &contract_id);
        env.mock_all_auths();

        let owner = Address::generate(&env);
        let backend_pubkey = BytesN::from_array(&env, &[1u8; 32]);
        client.set_backend_pubkey(&backend_pubkey);

        // Create a valid signature (mock)
        let signature = BytesN::from_array(&env, &[2u8; 64]);
        
        let due_date = env.ledger().timestamp() + 86400; // Tomorrow
        let invoice_id = client.mint(&owner, &1000, &due_date, &750, &signature);

        let invoice = client.get_invoice(&invoice_id);
        assert_eq!(invoice.owner, owner);
        assert_eq!(invoice.amount, 1000);
        assert_eq!(invoice.due_date, due_date);
        assert!(!invoice.is_repaid);
    }

    #[test]
    #[should_panic(expected = "INVOICE_EXPIRED")]
    fn test_mint_expired_invoice() {
        let env = Env::default();
        let contract_id = env.register(crate::InvoiceContract, ());
        let client = InvoiceContractClient::new(&env, &contract_id);
        env.mock_all_auths();

        let owner = Address::generate(&env);
        let backend_pubkey = BytesN::from_array(&env, &[1u8; 32]);
        client.set_backend_pubkey(&backend_pubkey);

        let signature = BytesN::from_array(&env, &[2u8; 64]);
        let past_date = env.ledger().timestamp(); // Current time, expires immediately

        client.mint(&owner, &1000, &past_date, &750, &signature);
    }

    #[test]
    #[should_panic(expected = "INVALID_SIGNATURE")]
    fn test_mint_invalid_signature() {
        let env = Env::default();
        let contract_id = env.register(crate::InvoiceContract, ());
        let client = InvoiceContractClient::new(&env, &contract_id);
        env.mock_all_auths();

        let owner = Address::generate(&env);
        let backend_pubkey = BytesN::from_array(&env, &[1u8; 32]);
        client.set_backend_pubkey(&backend_pubkey);

        let invalid_signature = BytesN::from_array(&env, &[99u8; 64]); // Invalid signature
        let due_date = env.ledger().timestamp() + 86400;

        client.mint(&owner, &1000, &due_date, &750, &invalid_signature);
    }

    #[test]
    fn test_repay_invoice() {
        let env = Env::default();
        let contract_id = env.register(crate::InvoiceContract, ());
        let client = InvoiceContractClient::new(&env, &contract_id);
        env.mock_all_auths();

        let owner = Address::generate(&env);
        let backend_pubkey = BytesN::from_array(&env, &[1u8; 32]);
        client.set_backend_pubkey(&backend_pubkey);

        let signature = BytesN::from_array(&env, &[2u8; 64]);
        let due_date = env.ledger().timestamp() + 86400;
        let invoice_id = client.mint(&owner, &1000, &due_date, &750, &signature);

        client.repay(&invoice_id);

        let invoice = client.get_invoice(&invoice_id);
        assert!(invoice.is_repaid);
    }

    #[test]
    #[should_panic(expected = "Invoice not found")]
    fn test_repay_nonexistent_invoice() {
        let env = Env::default();
        let contract_id = env.register(crate::InvoiceContract, ());
        let client = InvoiceContractClient::new(&env, &contract_id);

        client.repay(&999);
    }
}
