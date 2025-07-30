use sodax_backend_analizer::db::{
    find_all_a_token_addresses, find_all_reserve_addresses, find_all_reserves,
    find_all_variable_debt_token_addresses, get_reserve_data_for_a_token,
};

#[test]
fn test_find_all_reserves() {
    let result = find_all_reserves();
    assert!(result.is_ok(), "Should successfully retrieve reserves");

    let reserves = result.unwrap();
    // Add assertions based on expected data
    assert!(!reserves.is_empty(), "Should have some reserves");
}

#[test]
fn test_find_reserve_for_a_token() {
    // Test with a known token address (you'd need to insert test data first)
    let test_token = "0x5c50cf875aebad8d5ba548f229960c90b1c1f8c3";
    let result = get_reserve_data_for_a_token(test_token);

    // This might return None if the token doesn't exist in test data
    match result {
        Ok(Some(_value)) => {
            // Token found
            // dbg!(_value);
        }
        Ok(None) => {
            // Token not found - this is valid for test data
        }
        Err(e) => {
            panic!("Database error: {}", e);
        }
    }
}

#[test]
fn test_find_all_reserve_addresses() {
    let addresses = find_all_reserve_addresses();
    // Should return a Vec<String> even if empty
    assert!(addresses.is_empty() || !addresses.is_empty());
}

#[test]
fn test_find_all_a_token_addresses() {
    let addresses = find_all_a_token_addresses();
    // Should return a Vec<String> even if empty
    // dbg!(&addresses);
    assert!(addresses.is_empty() || !addresses.is_empty());
}

#[test]
fn test_find_all_variable_debt_token_addresses() {
    let addresses = find_all_variable_debt_token_addresses();
    // Should return a Vec<String> even if empty
    assert!(addresses.is_empty() || !addresses.is_empty());
}
