use sodax_backend_analizer::evm::{
  get_atoken_liquidity_index, get_balance_of, get_last_block, get_reserve_data, get_total_supply,
  get_variable_borrow_index, get_block_timestamp,
};

// Import common test utilities
mod common;
use common::{common_handler, common_handler_with_assertion};

const RESERVE_TOKEN_ADDRESS: &str = "0xe801ca34e19abcbfea12025378d19c4fbe250131";
const A_TOKEN_ADDRESS: &str = "0x5c50cf875aebad8d5ba548f229960c90b1c1f8c3";
const USER_ADDRESS: &str = "0xf754037f99af3d90a4db611d74966729b81a8a96";

// EVM tests
#[tokio::test]
async fn test_get_last_block() {
  let result = get_last_block().await;
  common_handler(
    result,
    "Last block retrieved successfully",
    "Failed to retrieve last block",
  );
}

#[tokio::test]
async fn test_get_balance_of() {
  // Test with valid Ethereum addresses
  let result = get_balance_of(A_TOKEN_ADDRESS, USER_ADDRESS, None).await;
  common_handler(
    result,
    "Balance retrieved successfully",
    "Failed to retrieve balance",
  );
}

#[tokio::test]
async fn test_get_balance_of_at_specific_block() {
  // Test balance at a specific block
  let token_address = "0x7ac26a7f46a4edc41586ebcadacb28b4020882c2";
  let user_address = "0x73135d19c488ea5b002e0e07135d992ff7e6f070";
  let block_number = 55349930;
  let expected_balance: u128 = 893954906014061389086;

  let result = get_balance_of(token_address, user_address, Some(block_number)).await;
  common_handler_with_assertion(
    result,
    expected_balance,
    &format!("Balance at block {} retrieved successfully", block_number),
    "Failed to retrieve balance at specific block",
  );
}

#[tokio::test]
async fn test_get_total_supply() {
  // Test with valid Ethereum address
  let result = get_total_supply(A_TOKEN_ADDRESS).await;
  common_handler(
    result,
    "Total supply retrieved successfully",
    "Failed to retrieve total supply",
  );
}

#[tokio::test]
async fn test_get_reserve_data() {
  let result = get_reserve_data(RESERVE_TOKEN_ADDRESS).await;
  common_handler(
    result,
    "Reserve data retrieved successfully",
    "Failed to retrieve reserve data",
  );
}

#[tokio::test]
async fn test_get_atoken_liquidity_index() {
  let result = get_atoken_liquidity_index(RESERVE_TOKEN_ADDRESS).await;
  common_handler(
    result,
    "AToken liquidity index retrieved successfully",
    "Failed to retrieve AToken liquidity index",
  );
}

#[tokio::test]
async fn test_get_variable_borrow_index() {
  let result = get_variable_borrow_index(RESERVE_TOKEN_ADDRESS).await;
  common_handler(
    result,
    "Variable borrow index retrieved successfully",
    "Failed to retrieve variable borrow index",
  );
}

#[tokio::test]
async fn test_get_block_timestamp() {
  let block_number: u64 = 1_000_000;
  let result = get_block_timestamp(block_number).await;
  common_handler(
    result,
    "Block timestamp retrieved successfully",
    "Failed to retrieve block timestamp",
  );
}
