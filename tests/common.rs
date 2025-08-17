// Common test utilities for integration tests
#[allow(dead_code)]
// pub const RESERVE_TOKEN_ADDRESS: &str = "0xe801ca34e19abcbfea12025378d19c4fbe250131";
pub const RESERVE_TOKEN_ADDRESS: &str = "0x14238d267557e9d799016ad635b53cd15935d290";
#[allow(dead_code)]
pub const A_TOKEN_ADDRESS: &str = "0x5c50cf875aebad8d5ba548f229960c90b1c1f8c3";
#[allow(dead_code)]
// pub const USER_ADDRESS: &str = "0xf754037f99af3d90a4db611d74966729b81a8a96";
pub const USER_ADDRESS: &str = "0x6d7b6956589c17B2755193a67BF2d4B68827E58A";
#[allow(dead_code)]
// pub const VARIABLE_DEBT_TOKEN_ADDRESS: &'static str = "0x19c023ff9c8105bf58e022c17a636b8e55ed8fe4";
pub const VARIABLE_DEBT_TOKEN_ADDRESS: &'static str = "0x96a4197803ac8b21a1b7aefe72e565c71a91a40f";

// Common handler for Result types in tests
pub fn common_handler<T, E: std::fmt::Display>(
    result: Result<T, E>,
    success_msg: &str,
    error_msg: &str,
) where
    T: std::fmt::Debug,
{
    match result {
        Ok(value) => {
            println!("✅ {}: {:?}", success_msg, value);
        }
        Err(e) => {
            println!("❌ {}: {}", error_msg, e);
        }
    }
}

pub fn common_vec_handler<T, E: std::fmt::Display>(
    result: Result<Vec<T>, E>,
    success_msg: &str,
    error_msg: &str,
) where
    T: std::fmt::Debug,
{
    match result {
        Ok(value) => {
            println!("✅ {}: {:?}", success_msg, value[0]);
        }
        Err(e) => {
            println!("❌ {}: {}", error_msg, e);
        }
    }
}
// Common handler for Option types in tests
#[allow(dead_code)]
pub fn common_option_handler<T>(option: Option<T>, found_msg: &str, not_found_msg: &str)
where
    T: std::fmt::Debug,
{
    match option {
        Some(value) => {
            println!("✅ {}: {:?}", found_msg, value);
        }
        None => {
            println!("ℹ️ {}", not_found_msg);
        }
    }
}

// Common handler for Result<Option<T>, E> types in tests
#[allow(dead_code)]
pub fn common_result_option_handler<T, E: std::fmt::Debug>(
    result: Result<Option<T>, E>,
    found_msg: &str,
    not_found_msg: &str,
    error_msg: &str,
) where
    T: std::fmt::Debug,
{
    match result {
        Ok(Some(value)) => {
            println!("✅ {}: {:?}", found_msg, value);
        }
        Ok(None) => {
            println!("ℹ️ {}", not_found_msg);
        }
        Err(e) => {
            println!("❌ {}: {:?}", error_msg, e);
        }
    }
}
