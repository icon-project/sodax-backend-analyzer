use sodax_backend_analizer::helpers::{
    calculate_user_borrow_amount, calculate_user_supply_amount, calculate_token_borrow_amount,
    calculate_token_supply_amount,
};
use sodax_backend_analizer::validators::{
    validate_token_borrow_amount, validate_token_supply_amount, validate_user_borrow_amount,
    validate_user_supply_amount,
};

// Import common test utilities
mod common;
use common::{
    common_handler,
    // A_TOKEN_ADDRESS,
    RESERVE_TOKEN_ADDRESS,
    USER_ADDRESS,
};

#[tokio::test]
async fn test_calculate_user_supply_amount() {
    let result = calculate_user_supply_amount(USER_ADDRESS, RESERVE_TOKEN_ADDRESS).await;

    common_handler(
        result,
        "user supply calculated successfully",
        "Failed to calculate aToken balance for user",
    );
}

#[tokio::test]
async fn test_calculate_user_borrow_amount() {
    let result = calculate_user_borrow_amount(USER_ADDRESS, RESERVE_TOKEN_ADDRESS).await;

    common_handler(
        result,
        "user borrow calculated successfully",
        "Failed to calculate aToken balance for user",
    );
}

#[tokio::test]
async fn test_calculate_token_supply_amount() {
    let result = calculate_token_supply_amount(RESERVE_TOKEN_ADDRESS).await;

    common_handler(
        result,
        "token supply amount calculated successfully",
        "Failed to calculate supply amount for token",
    );
}

#[tokio::test]
async fn test_calculate_token_borrow_amount() {
    let result = calculate_token_borrow_amount(RESERVE_TOKEN_ADDRESS).await;

    common_handler(
        result,
        "token borrow amount calculated successfully",
        "Failed to calculate borrow amount for token",
    );
}

#[tokio::test]
async fn test_validate_user_supply_amount() {
    let result = validate_user_supply_amount(USER_ADDRESS, RESERVE_TOKEN_ADDRESS).await;
    common_handler(
        result,
        "user supply amount validated successfully",
        "Failed to validate user supply amount",
    );
}

#[tokio::test]
async fn test_validate_user_borrow_amount() {
    let result = validate_user_borrow_amount(USER_ADDRESS, RESERVE_TOKEN_ADDRESS).await;
    common_handler(
        result,
        "user borrow amount validated successfully",
        "Failed to validate user borrow amount",
    );
}

#[tokio::test]
async fn test_validate_token_supply_amount() {
    let result = validate_token_supply_amount(RESERVE_TOKEN_ADDRESS).await;
    common_handler(
        result,
        "token supply amount validated successfully",
        "Failed to validate token supply amount",
    );
}

#[tokio::test]
async fn test_validate_token_borrow_amount() {
    let result = validate_token_borrow_amount(RESERVE_TOKEN_ADDRESS).await;
    common_handler(
        result,
        "token borrow amount validated successfully",
        "Failed to validate token borrow amount",
    );
}
