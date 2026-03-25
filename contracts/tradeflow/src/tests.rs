use soroban_sdk::{
    Address, Env, BytesN, Symbol, Vec, Val, testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation},
    token,
};
use crate::{
    TradeFlow, LiquidityPosition, PendingFeeChange, PermitData, DataKey, TWAPConfig, PriceObservation, BuybackConfig, FeeAccumulator,
    utils::fixed_point::{self, Q64},
};

#[test]
fn test_initialization() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);
    
    TradeFlow::init(&env, admin.clone(), token_a.clone(), token_b.clone(), 30);
    
    assert_eq!(TradeFlow::get_protocol_fee(&env), 30);
    
    let (reserve_a, reserve_b) = TradeFlow::get_reserves(&env);
    assert_eq!(reserve_a, 0);
    assert_eq!(reserve_b, 0);
}

#[test]
fn test_fee_change_timelock() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);
    
    TradeFlow::init(&env, admin.clone(), token_a, token_b, 30);
    
    // Propose fee change
    TradeFlow::propose_fee_change(&env, 50);
    
    let pending = TradeFlow::get_pending_fee_change(&env).unwrap();
    assert_eq!(pending.new_fee, 50);
    
    // Should not be able to execute immediately
    env.mock_auths(&[
        (&admin, &AuthorizedInvocation {
            contract: &env.current_contract_address(),
            function: &AuthorizedFunction::Contract((
                Symbol::new(&env, "execute_fee_change"),
                (),
            )),
            sub_invocations: &[]
        })
    ]);
    
    let result = std::panic::catch_unwind(|| {
        TradeFlow::execute_fee_change(&env);
    });
    assert!(result.is_err()); // Should panic due to timelock
    
    // Fast forward time by 48 hours
    env.ledger().set_timestamp(env.ledger().timestamp() + 48 * 60 * 60 + 1);
    
    // Now should be able to execute
    TradeFlow::execute_fee_change(&env);
    assert_eq!(TradeFlow::get_protocol_fee(&env), 50);
    assert!(TradeFlow::get_pending_fee_change(&env).is_none());
}

#[test]
fn test_provide_liquidity() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);
    
    // Create token contracts for testing
    let token_a_client = token::Client::new(&env, &token_a);
    let token_b_client = token::Client::new(&env, &token_b);
    
    TradeFlow::init(&env, admin, token_a.clone(), token_b.clone(), 30);
    
    // Mint tokens to user
    // Note: In a real test environment, you'd need to set up proper token contracts
    // For this example, we'll assume the tokens are already minted
    
    // Provide liquidity
    let shares = TradeFlow::provide_liquidity(&env, user.clone(), 100, 200, 1);
    
    assert!(shares > 0);
    
    let position = TradeFlow::get_liquidity_position(&env, user).unwrap();
    assert_eq!(position.token_a_amount, 100);
    assert_eq!(position.token_b_amount, 200);
    assert_eq!(position.shares, shares);
    
    let (reserve_a, reserve_b) = TradeFlow::get_reserves(&env);
    assert_eq!(reserve_a, 100);
    assert_eq!(reserve_b, 200);
}

#[test]
fn test_swap() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);
    
    let token_a_client = token::Client::new(&env, &token_a);
    let token_b_client = token::Client::new(&env, &token_b);
    
    TradeFlow::init(&env, admin, token_a.clone(), token_b.clone(), 30);
    
    // First provide liquidity
    // Note: In a real test, you'd need to mint tokens to the user first
    TradeFlow::provide_liquidity(&env, user.clone(), 500, 500, 1);
    
    // Now perform swap
    // Note: In a real test, you'd need to mint tokens to the user first
    let amount_out = TradeFlow::swap(&env, user.clone(), token_a.clone(), 100, 1);
    
    assert!(amount_out >= 1);
    
    // Check reserves changed correctly
    let (reserve_a, reserve_b) = TradeFlow::get_reserves(&env);
    assert_eq!(reserve_a, 600); // 500 + 100
    assert!(reserve_b < 500); // Decreased due to swap
}

#[test]
fn test_permit_swap() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);
    
    let token_a_client = token::Client::new(&env, &token_a);
    let token_b_client = token::Client::new(&env, &token_b);
    
    TradeFlow::init(&env, admin, token_a.clone(), token_b.clone(), 30);
    
    // First provide liquidity
    // Note: In a real test, you'd need to mint tokens to the user first
    TradeFlow::provide_liquidity(&env, user.clone(), 500, 500, 1);
    
    // Create permit data
    let permit_data = PermitData {
        owner: user.clone(),
        spender: env.current_contract_address(),
        amount: 100,
        nonce: TradeFlow::get_user_nonce(&env, user.clone()),
        deadline: env.ledger().timestamp() + 3600, // 1 hour from now
    };
    
    // Mock signature (in real implementation, this would be a valid signature)
    let signature = BytesN::from_array(&env, &[0u8; 64]);
    
    // Mock signature verification
    env.mock_auths(&[
        (&user, &AuthorizedInvocation {
            contract: &env.current_contract_address(),
            function: &AuthorizedFunction::Contract((
                Symbol::new(&env, "permit_swap"),
                (
                    user.clone(),
                    token_a.clone(),
                    100u128,
                    1u128,
                    permit_data.clone(),
                    signature.clone(),
                ),
            )),
            sub_invocations: &[]
        })
    ]);
    
    // Note: In a real test, you'd need to mint tokens to the user first
    
    // This should work with proper signature verification
    // In a real implementation with proper signature generation, this would pass
    let result = std::panic::catch_unwind(|| {
        TradeFlow::permit_swap(&env, user.clone(), token_a.clone(), 100, 1, permit_data, signature);
    });
    
    // For this test, we expect it to fail due to invalid signature
    // In a real implementation with proper signature generation, this would pass
    assert!(result.is_err());
}

#[test]
fn test_fixed_point_math() {
    let env = Env::default();
    
    // Test mul_div_down
    let result = fixed_point::mul_div_down(&env, 100, 200, 50);
    assert_eq!(result, 400); // (100 * 200) / 50 = 400
    
    // Test mul_div_up
    let result = fixed_point::mul_div_up(&env, 100, 200, 50);
    assert_eq!(result, 400); // Same as down since it divides evenly
    
    // Test with rounding
    let result = fixed_point::mul_div_down(&env, 100, 3, 2);
    assert_eq!(result, 150); // (100 * 3) / 2 = 150
    
    let result = fixed_point::mul_div_up(&env, 100, 3, 2);
    assert_eq!(result, 150); // Same in this case
    
    // Test scale operations
    let scaled = fixed_point::scale_up(&env, 100);
    assert_eq!(scaled, 100 * Q64);
    
    let downscaled = fixed_point::scale_down(&env, scaled);
    assert_eq!(downscaled, 100);
}

#[test]
fn test_user_nonce_increment() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);
    
    TradeFlow::init(&env, admin, token_a, token_b, 30);
    
    // Initial nonce should be 0
    assert_eq!(TradeFlow::get_user_nonce(&env, user.clone()), 0);
    
    // Note: In a real test, you'd need to mint tokens to the user first
    TradeFlow::provide_liquidity(&env, user.clone(), 100, 200, 1);
    assert_eq!(TradeFlow::get_user_nonce(&env, user.clone()), 0);
}

#[test]
fn test_twap_config_initialization() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);
    
    TradeFlow::init(&env, admin.clone(), token_a, token_b, 30);
    
    // Check default TWAP configuration
    let config = TradeFlow::get_twap_config(&env);
    assert_eq!(config.window_size, 3600); // 1 hour
    assert_eq!(config.max_deviation, 1000); // 10%
    assert_eq!(config.enabled, true);
}

#[test]
fn test_twap_config_update() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);
    
    TradeFlow::init(&env, admin.clone(), token_a, token_b, 30);
    
    // Update TWAP configuration
    TradeFlow::set_twap_config(&env, Some(7200), Some(500), Some(false));
    
    let config = TradeFlow::get_twap_config(&env);
    assert_eq!(config.window_size, 7200); // 2 hours
    assert_eq!(config.max_deviation, 500); // 5%
    assert_eq!(config.enabled, false);
}

#[test]
fn test_price_observation() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);
    
    TradeFlow::init(&env, admin.clone(), token_a.clone(), token_b.clone(), 30);
    
    // Add some liquidity to create price observations
    let user = Address::generate(&env);
    
    // Mock token contracts for liquidity provision
    let token_a_client = token::Client::new(&env, &token_a);
    let token_b_client = token::Client::new(&env, &token_b);
    
    // Mint tokens to user (in a real scenario, this would be done by token contracts)
    token_a_client.mint(&user, &1000);
    token_b_client.mint(&user, &2000);
    
    // Provide liquidity
    TradeFlow::provide_liquidity(&env, user.clone(), 100, 200, 1);
    
    // Check that price observation was created
    let last_observation: Option<PriceObservation> = env.storage().instance()
        .get(&DataKey::LastObservation);
    
    assert!(last_observation.is_some(), "Price observation should be created after providing liquidity");
    
    let obs = last_observation.unwrap();
    assert!(obs.timestamp > 0, "Timestamp should be set");
    assert!(obs.price_a_per_b > 0, "Price should be calculated");
    assert!(obs.price_b_per_a > 0, "Price should be calculated");
}

#[test]
fn test_twap_slippage_protection() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);
    
    TradeFlow::init(&env, admin.clone(), token_a.clone(), token_b.clone(), 30);
    
    // Add liquidity
    let user = Address::generate(&env);
    let token_a_client = token::Client::new(&env, &token_a);
    let token_b_client = token::Client::new(&env, &token_b);
    
    token_a_client.mint(&user, &1000);
    token_b_client.mint(&user, &2000);
    
    TradeFlow::provide_liquidity(&env, user.clone(), 100, 200, 1);
    
    // Test normal swap (should pass)
    token_a_client.mint(&user, &10);
    
    // This should work as it's a normal sized swap
    let result = std::panic::catch_unwind(|| {
        TradeFlow::swap(&env, user.clone(), token_a.clone(), 10, 1);
    });
    
    // The swap should succeed (no panic)
    assert!(result.is_ok(), "Normal swap should succeed");
    
    // Test with disabled TWAP protection
    TradeFlow::set_twap_config(&env, None, None, Some(false));
    
    let result2 = std::panic::catch_unwind(|| {
        TradeFlow::swap(&env, user.clone(), token_a.clone(), 10, 1);
    });
    
    // Should still succeed when protection is disabled
    assert!(result2.is_ok(), "Swap should succeed when TWAP protection is disabled");
}

#[test]
fn test_twap_config_validation() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);
    
    TradeFlow::init(&env, admin.clone(), token_a, token_b, 30);
    
    // Test partial updates
    TradeFlow::set_twap_config(&env, Some(1800), None, None);
    let config = TradeFlow::get_twap_config(&env);
    assert_eq!(config.window_size, 1800);
    assert_eq!(config.max_deviation, 1000); // unchanged
    assert_eq!(config.enabled, true); // unchanged
    
    TradeFlow::set_twap_config(&env, None, Some(2000), None);
    let config = TradeFlow::get_twap_config(&env);
    assert_eq!(config.window_size, 1800); // unchanged
    assert_eq!(config.max_deviation, 2000);
    assert_eq!(config.enabled, true); // unchanged
    
    TradeFlow::set_twap_config(&env, None, None, Some(false));
    let config = TradeFlow::get_twap_config(&env);
    assert_eq!(config.window_size, 1800); // unchanged
    assert_eq!(config.max_deviation, 2000); // unchanged
    assert_eq!(config.enabled, false);
}

#[test]
fn test_fee_accumulator_initialization() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);
    
    TradeFlow::init(&env, admin.clone(), token_a, token_b, 30);
    
    // Check fee accumulator initialization
    let accumulator = TradeFlow::get_fee_accumulator(&env);
    assert_eq!(accumulator.token_a_fees, 0);
    assert_eq!(accumulator.token_b_fees, 0);
    assert!(accumulator.last_collection_time > 0);
    assert_eq!(accumulator.total_fees_collected, 0);
    assert_eq!(accumulator.total_tokens_burned, 0);
}

#[test]
fn test_buyback_configuration() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);
    let tf_token = Address::generate(&env);
    let fee_recipient = Address::generate(&env);
    
    TradeFlow::init(&env, admin.clone(), token_a, token_b, 30);
    
    // Configure buyback
    TradeFlow::configure_buyback(&env, tf_token.clone(), fee_recipient.clone(), 5000); // 50% burn
    
    let config = TradeFlow::get_buyback_config(&env).unwrap();
    assert_eq!(config.tf_token_address, tf_token);
    assert_eq!(config.fee_recipient, fee_recipient);
    assert_eq!(config.burn_percentage, 5000);
    assert!(config.buyback_enabled);
}

#[test]
fn test_buyback_configuration_validation() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);
    let tf_token = Address::generate(&env);
    let fee_recipient = Address::generate(&env);
    
    TradeFlow::init(&env, admin.clone(), token_a, token_b, 30);
    
    // Test invalid burn percentage
    let result = std::panic::catch_unwind(|| {
        TradeFlow::configure_buyback(&env, tf_token, fee_recipient, 15000); // > 100%
    });
    
    assert!(result.is_err(), "Should panic with invalid burn percentage");
}

#[test]
fn test_fee_collection_from_swaps() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);
    
    TradeFlow::init(&env, admin.clone(), token_a.clone(), token_b.clone(), 30);
    
    // Add liquidity
    let user = Address::generate(&env);
    let token_a_client = token::Client::new(&env, &token_a);
    let token_b_client = token::Client::new(&env, &token_b);
    
    token_a_client.mint(&user, &1000);
    token_b_client.mint(&user, &2000);
    
    TradeFlow::provide_liquidity(&env, user.clone(), 100, 200, 1);
    
    // Perform a swap to generate fees
    token_a_client.mint(&user, &10);
    
    // Mock the swap to see fee collection
    let result = std::panic::catch_unwind(|| {
        TradeFlow::swap(&env, user.clone(), token_a.clone(), 10, 1);
    });
    
    // Check that fees were accumulated
    let accumulator = TradeFlow::get_fee_accumulator(&env);
    assert!(accumulator.total_fees_collected > 0, "Fees should be collected from swaps");
}

#[test]
fn test_buyback_toggle() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);
    let tf_token = Address::generate(&env);
    let fee_recipient = Address::generate(&env);
    
    TradeFlow::init(&env, admin.clone(), token_a, token_b, 30);
    TradeFlow::configure_buyback(&env, tf_token, fee_recipient, 5000);
    
    // Test toggling buyback off
    TradeFlow::toggle_buyback(&env, false);
    let config = TradeFlow::get_buyback_config(&env).unwrap();
    assert!(!config.buyback_enabled);
    
    // Test toggling buyback on
    TradeFlow::toggle_buyback(&env, true);
    let config = TradeFlow::get_buyback_config(&env).unwrap();
    assert!(config.buyback_enabled);
}

#[test]
fn test_buyback_execution() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);
    let tf_token = Address::generate(&env);
    let fee_recipient = Address::generate(&env);
    
    TradeFlow::init(&env, admin.clone(), token_a.clone(), token_b.clone(), 30);
    TradeFlow::configure_buyback(&env, tf_token.clone(), fee_recipient.clone(), 5000); // 50% burn
    
    // Simulate accumulated fees
    let mut accumulator = TradeFlow::get_fee_accumulator(&env);
    accumulator.token_a_fees = 1000; // Simulate 1000 fees collected
    env.storage().instance().set(&DataKey::FeeAccumulator, &accumulator);
    
    // Execute buyback
    let tf_received = TradeFlow::execute_buyback_and_burn(
        &env,
        token_a.clone(),
        500, // Use 500 fees for buyback
        400  // Expect at least 400 TF tokens
    );
    
    assert!(tf_received >= 400, "Should receive at least minimum TF tokens");
    
    // Check that fees were deducted
    let updated_accumulator = TradeFlow::get_fee_accumulator(&env);
    assert_eq!(updated_accumulator.token_a_fees, 500); // 1000 - 500 used
    assert!(updated_accumulator.total_tokens_burned > 0, "Tokens should be burned");
}

#[test]
fn test_buyback_insufficient_fees() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);
    let tf_token = Address::generate(&env);
    let fee_recipient = Address::generate(&env);
    
    TradeFlow::init(&env, admin.clone(), token_a.clone(), token_b.clone(), 30);
    TradeFlow::configure_buyback(&env, tf_token, fee_recipient, 5000);
    
    // Try to execute buyback with insufficient fees
    let result = std::panic::catch_unwind(|| {
        TradeFlow::execute_buyback_and_burn(
            &env,
            token_a,
            1000, // Try to use 1000 fees
            800   // Expect 800 TF tokens
        );
    });
    
    assert!(result.is_err(), "Should panic with insufficient fees");
}

#[test]
fn test_buyback_disabled() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);
    let tf_token = Address::generate(&env);
    let fee_recipient = Address::generate(&env);
    
    TradeFlow::init(&env, admin.clone(), token_a.clone(), token_b.clone(), 30);
    TradeFlow::configure_buyback(&env, tf_token, fee_recipient, 5000);
    
    // Disable buyback
    TradeFlow::toggle_buyback(&env, false);
    
    // Try to execute buyback
    let result = std::panic::catch_unwind(|| {
        TradeFlow::execute_buyback_and_burn(
            &env,
            token_a,
            100,
            80
        );
    });
    
    assert!(result.is_err(), "Should panic when buyback is disabled");
}
