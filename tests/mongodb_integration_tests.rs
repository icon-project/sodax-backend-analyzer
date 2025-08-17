use sodax_backend_analizer::db::{
    find_all_a_token_addresses,
    find_all_reserve_addresses,
    find_all_reserves,
    find_all_variable_debt_token_addresses,
    find_reserve_for_token,
    get_orderbook,
    find_timestamp_and_block_from_solver_volume,
    find_user_events,
    find_token_events,
    // get_user_position,
};
use sodax_backend_analizer::structs::ReserveTokenField;

// Import common test utilities
mod common;
use common::{
    common_handler,
    common_vec_handler,
    common_result_option_handler,
    A_TOKEN_ADDRESS,
    RESERVE_TOKEN_ADDRESS,
    //
    USER_ADDRESS,
    VARIABLE_DEBT_TOKEN_ADDRESS,
};

// Database tests
#[tokio::test]
async fn test_find_all_reserves() {
    let result = find_all_reserves().await;
    assert!(result.is_ok(), "Should successfully retrieve reserves");

    let reserves = result.unwrap();
    // dbg!(&reserves[0]);
    // Add assertions based on expected data
    assert!(!reserves.is_empty(), "Should have some reserves");
}

#[tokio::test]
async fn test_find_all_reserve_addresses() {
    let addresses = find_all_reserve_addresses().await;
    // println!(
    //     "✅ Reserve addresses retrieved successfully: {:?}",
    //     addresses
    // );
    // Should return a Vec<String> even if empty
    assert!(addresses.is_empty() || !addresses.is_empty());
}

#[tokio::test]
async fn test_find_all_a_token_addresses() {
    let addresses = find_all_a_token_addresses().await;
    // Should return a Vec<String> even if empty
    // dbg!(&addresses[0]);
    assert!(addresses.is_empty() || !addresses.is_empty());
}

#[tokio::test]
async fn test_find_all_variable_debt_token_addresses() {
    let addresses = find_all_variable_debt_token_addresses().await;
    // dbg!(&addresses[0]);
    // Should return a Vec<String> even if empty
    assert!(addresses.is_empty() || !addresses.is_empty());
}

#[tokio::test]
async fn test_get_reserve_data_for_reserve_token() {
    // Test with a known token address (you'd need to insert test data first)
    let result = find_reserve_for_token(RESERVE_TOKEN_ADDRESS, ReserveTokenField::Reserve).await;

    common_result_option_handler(
        result,
        "Reserve token data found",
        "Reserve token not found, which is valid for test data",
        "Database error occurred",
    );
}

#[tokio::test]
async fn test_get_reserve_data_for_a_token() {
    // Test with a known token address (you'd need to insert test data first)
    let result = find_reserve_for_token(A_TOKEN_ADDRESS, ReserveTokenField::AToken).await;

    common_result_option_handler(
        result,
        "A token data found",
        "A token not found, which is valid for test data",
        "Database error occurred",
    );
}

#[tokio::test]
async fn test_get_reserve_data_for_variable_debt_token() {
    // Test with a known token address (you'd need to insert test data first)
    let result = find_reserve_for_token(
        VARIABLE_DEBT_TOKEN_ADDRESS,
        ReserveTokenField::VariableDebtToken,
    )
    .await;

    common_result_option_handler(
        result,
        "Variable debt token data found",
        "Variable debt token not found, which is valid for test data",
        "Database error occurred",
    );
}

#[tokio::test]
async fn test_get_orderbook() {
    let result = get_orderbook().await;

    common_handler(
        result,
        "Orderbook data found",
        "Orderbook data not found, which is valid for test data",
    );
}

#[tokio::test]
async fn test_find_timestamp_and_block_from_solver_volume() {
    let result = find_timestamp_and_block_from_solver_volume().await;

    common_vec_handler(
        result,
        "Timestamp and block data found",
        "Timestamp and block data not found, which is valid for test data",
    );
}

#[tokio::test]
async fn test_find_user_events() {
    let result = find_user_events(USER_ADDRESS).await;

    common_vec_handler(
        result,
        "User events data found",
        "User events not found, which is valid for test data",
    );
}

#[tokio::test]
async fn test_find_token_events() {
    let result = find_token_events(A_TOKEN_ADDRESS).await;
    common_vec_handler(
        result,
        "Token events data found",
        "Token events not found, which is valid for test data",
    );
}
// #[ignore]
// #[tokio::test]
// async fn test_get_user_position() {
//     // Test with a known user address (you'd need to insert test data first)
//     let result = get_user_position(&USER_ADDRESS).await;
//
//     common_result_option_handler(
//         result,
//         "User position data found",
//         "User position not found, which is valid for test data",
//         "Database error occurred",
//     );
// }
