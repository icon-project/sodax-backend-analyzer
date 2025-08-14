use crate::constants::RAY;
use crate::db::{find_reserve_for_token, get_user_position};
use crate::evm::{get_atoken_liquidity_index, get_variable_borrow_index};
use crate::models::UserAssetPositionDocument;
use crate::structs::ReserveTokenField;
use crate::functions::{ray_div, ray_mul};
use primitive_types::U256;
// use mongodb::bson::Decimal128;

// fn decimal128_to_u64_blocknumber(d: Decimal128) -> u64 {
//     // Convert Decimal128 to u64
//     d.to_string()
//         .parse::<u64>()
//         .expect("Failed to parse Decimal128 to u64")
// }

pub async fn find_user_scaled_position(
    user_address: &str,
    reserve_address: &str,
) -> Result<UserAssetPositionDocument, Box<dyn std::error::Error>> {
    let reserve_data = get_user_position(user_address).await?;

    // Find the position for the specific reserve
    let position = reserve_data
        .positions
        .iter()
        .find(|position| position.reserveAddress == reserve_address)
        .ok_or("No position found for the specified reserve")?;
    Ok(position.clone())
}

fn calculate_real_balance(
    scaled_balance: u128,
    liquidity_index: u128,
) -> Result<u128, Box<dyn std::error::Error>> {
    // Aave formula: scaled_balance * liquidity_index / RAY

    let sb_u256 = U256::from(scaled_balance);
    let i_u256 = U256::from(liquidity_index);
    let intermidiate = ray_mul(sb_u256, i_u256).unwrap();
    let real_balance =
        ray_div(intermidiate, U256::from(RAY)).map_err(|e| format!("Math error: {:?}", e))?;
    Ok(real_balance
        .try_into()
        .map_err(|_| "Failed to convert real balance from U256 to u128")?)
}

pub async fn calculate_user_supply_amount(
    user_address: &str,
    reserve_address: &str,
) -> Result<u128, Box<dyn std::error::Error>> {
    let index = get_atoken_liquidity_index(reserve_address).await?;

    // Find the position for the specific token
    match find_user_scaled_position(user_address, reserve_address).await {
        Ok(position) => {
            // Convert Decimal128 to u128 for calculation
            let a_token_balance = position
                .aTokenBalance
                .to_string()
                .parse::<u128>()
                .map_err(|_| "Failed to parse aToken balance")?;

            let real_balance = calculate_real_balance(a_token_balance, index)?;
            Ok(real_balance)
        }
        Err(_) => {
            // User has no position for this reserve, return 0
            Ok(0)
        }
    }
}

pub async fn calculate_user_borrow_amount(
    user_address: &str,
    reserve_address: &str,
) -> Result<u128, Box<dyn std::error::Error>> {
    let index = get_variable_borrow_index(reserve_address).await?;

    // Find the position for the specific token
    match find_user_scaled_position(user_address, reserve_address).await {
        Ok(position) => {
            // Convert Decimal128 to u128 for calculation
            let a_token_balance = position
                .variableDebtTokenBalance
                .to_string()
                .parse::<u128>()
                .map_err(|_| "Failed to parse variable debt token balance")?;

            let real_balance = calculate_real_balance(a_token_balance, index)?;
            Ok(real_balance)
        }
        Err(_) => {
            // User has no position for this reserve, return 0
            Ok(0)
        }
    }
}
pub async fn get_token_scaled_supply_amount(
    reserve_address: &str,
) -> Result<u128, Box<dyn std::error::Error>> {
    let token_data = find_reserve_for_token(reserve_address, ReserveTokenField::Reserve)
        .await?
        .ok_or("No reserve data found for the specified reserve address")?;

    let scaled_balance = token_data
        .totalATokenBalance
        .to_string()
        .parse::<u128>()
        .map_err(|_| "Failed to parse total supply")?;

    Ok(scaled_balance)
}

pub async fn calculate_token_supply_amount(
    reserve_address: &str,
) -> Result<u128, Box<dyn std::error::Error>> {
    let index = get_atoken_liquidity_index(reserve_address).await?;

    let scaled_balance = get_token_scaled_supply_amount(reserve_address).await?;

    let real_balance = calculate_real_balance(scaled_balance, index)?;

    Ok(real_balance)
}

pub async fn get_token_scaled_borrow_amount(
    reserve_address: &str,
) -> Result<u128, Box<dyn std::error::Error>> {
    let token_data = find_reserve_for_token(reserve_address, ReserveTokenField::Reserve)
        .await?
        .ok_or("No reserve data found for the specified reserve address")?;

    let scaled_balance = token_data
        .totalVariableDebtTokenBalance
        .to_string()
        .parse::<u128>()
        .map_err(|_| "Failed to parse total borrow")?;

    Ok(scaled_balance)
}

pub async fn calculate_token_borrow_amount(
    reserve_address: &str,
) -> Result<u128, Box<dyn std::error::Error>> {
    let index = get_variable_borrow_index(reserve_address).await?;

    let scaled_balance = get_token_scaled_borrow_amount(reserve_address).await?;

    let real_balance = calculate_real_balance(scaled_balance, index)?;

    Ok(real_balance)
}

pub fn compare_and_report_diff(
    calculated_amount: u128,
    on_chain_amount: u128,
    description: &str,
) -> String {
    if calculated_amount == on_chain_amount {
        format!("✅ {} amounts match: {}", description, calculated_amount)
    } else {
        let diff = if calculated_amount > on_chain_amount {
            calculated_amount - on_chain_amount
        } else {
            on_chain_amount - calculated_amount
        };
        let percentage = (diff as f64 / on_chain_amount as f64) * 100.0;

        // main condition to define an acceptable difference
        // the diff is below an acceptable value
        // this value is 1000000 wei
        if diff < 1_000_000 {
            return format!(
                "⚠️ Minor mismatch for {}: calculated = {}, on-chain = {}, diff = {} ({:.4})%",
                description, calculated_amount, on_chain_amount, diff, percentage
            );
        }

        // if the diff is above the main condition, it will
        // still be considered a minor mismatch if the
        // percentage is below 0.01%
        if percentage < 0.01 {
            return format!(
                "⚠️ Minor mismatch for {}: calculated = {}, on-chain = {}, diff = {} ({:.4})%",
                description, calculated_amount, on_chain_amount, diff, percentage
            );
        }
        format!(
            "❌ Mismatch for {}: calculated = {}, on-chain = {}, diff = {} ({:.4})%",
            description, calculated_amount, on_chain_amount, diff, percentage
        )
    }
}
