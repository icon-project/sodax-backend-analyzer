use crate::constants::RAY;
use crate::db::{find_reserve_for_token, get_user_position, ReserveTokenField};
use crate::evm::{
    get_atoken_liquidity_index, get_balance_of, get_total_supply, get_variable_borrow_index,
};
use crate::models::UserAssetPositionDocument;

#[derive(Debug, Clone)]
pub struct EntryState {
    pub database_amount: u128,
    pub on_chain_amount: u128,
    pub difference: u128,
    pub percentage: f64,
}

impl EntryState {
    pub fn new(database_amount: u128, on_chain_amount: u128) -> Self {
        let difference = if database_amount > on_chain_amount {
            database_amount - on_chain_amount
        } else {
            on_chain_amount - database_amount
        };
        let percentage = (difference as f64 / on_chain_amount as f64) * 100.0;

        EntryState {
            database_amount,
            on_chain_amount,
            difference,
            percentage,
        }
    }
}

async fn find_user_position(
    user_address: &str,
    reserve_address: &str,
) -> Result<UserAssetPositionDocument, Box<dyn std::error::Error>> {
    let reserve_data = match get_user_position(user_address).await? {
        Some(data) => data,
        None => return Err("No user position found for user".into()),
    };

    // Find the position for the specific reserve
    let position = reserve_data
        .positions
        .iter()
        .find(|position| position.reserveAddress == reserve_address)
        .ok_or("No position found for the specified reserve")?;
    Ok(position.clone())
}

fn calculate_real_balance(scaled_balance: u128, liquidity_index: u128) -> u128 {
    // Aave formula: scaled_balance * liquidity_index / RAY
    // Use f64 for intermediate calculations to handle large numbers
    let scaled_balance_f64 = scaled_balance as f64;
    let liquidity_index_f64 = liquidity_index as f64;
    let ray_f64 = RAY as f64;

    let real_balance_f64 = (scaled_balance_f64 * liquidity_index_f64) / ray_f64;
    real_balance_f64 as u128
}

pub async fn calculate_user_supply_amount(
    user_address: &str,
    reserve_address: &str,
) -> Result<u128, Box<dyn std::error::Error>> {
    let index = get_atoken_liquidity_index(reserve_address).await?;

    // Find the position for the specific token
    let position = find_user_position(user_address, reserve_address).await?;

    // Convert Decimal128 to u128 for calculation
    let a_token_balance = position
        .aTokenBalance
        .to_string()
        .parse::<u128>()
        .map_err(|_| "Failed to parse aToken balance")?;

    let real_balance = calculate_real_balance(a_token_balance, index);

    Ok(real_balance)
}

pub async fn calculate_user_borrow_amount(
    user_address: &str,
    reserve_address: &str,
) -> Result<u128, Box<dyn std::error::Error>> {
    let index = get_variable_borrow_index(reserve_address).await?;

    // Find the position for the specific token
    let position = find_user_position(user_address, reserve_address).await?;

    // Convert Decimal128 to u128 for calculation
    let a_token_balance = position
        .variableDebtTokenBalance
        .to_string()
        .parse::<u128>()
        .map_err(|_| "Failed to parse variable debt token balance")?;

    let real_balance = calculate_real_balance(a_token_balance, index);

    Ok(real_balance)
}

pub async fn get_token_supply_amount(
    reserve_address: &str,
) -> Result<u128, Box<dyn std::error::Error>> {
    let index = get_atoken_liquidity_index(reserve_address).await?;

    let token_data = find_reserve_for_token(reserve_address, ReserveTokenField::Reserve).await?;

    let scaled_balance = token_data
        .totalATokenBalance
        .to_string()
        .parse::<u128>()
        .map_err(|_| "Failed to parse total supply")?;

    let real_balance = calculate_real_balance(scaled_balance, index);

    Ok(real_balance)
}

pub async fn get_token_borrow_amount(
    reserve_address: &str,
) -> Result<u128, Box<dyn std::error::Error>> {
    let index = get_variable_borrow_index(reserve_address).await?;

    let token_data = find_reserve_for_token(reserve_address, ReserveTokenField::Reserve).await?;

    let scaled_balance = token_data
        .totalVariableDebtTokenBalance
        .to_string()
        .parse::<u128>()
        .map_err(|_| "Failed to parse total supply")?;

    let real_balance = calculate_real_balance(scaled_balance, index);

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

pub async fn validate_user_supply_amount(
    user_address: &str,
    reserve_address: &str,
) -> Result<EntryState, Box<dyn std::error::Error>> {
    let calculated_amount = calculate_user_supply_amount(user_address, reserve_address).await?;

    let token_data = find_reserve_for_token(reserve_address, ReserveTokenField::Reserve).await?;

    let a_token_address = token_data.aTokenAddress;
    let on_chain_amount = get_balance_of(&a_token_address, user_address).await?;

    let result = EntryState::new(calculated_amount, on_chain_amount);
    Ok(result)
}

pub async fn validate_user_borrow_amount(
    user_address: &str,
    reserve_address: &str,
) -> Result<EntryState, Box<dyn std::error::Error>> {
    let calculated_amount = calculate_user_borrow_amount(user_address, reserve_address).await?;

    let token_data = find_reserve_for_token(reserve_address, ReserveTokenField::Reserve).await?;

    let variable_debt_token_address = token_data.variableDebtTokenAddress;

    let on_chain_amount = get_balance_of(&variable_debt_token_address, user_address).await?;

    let result = EntryState::new(calculated_amount, on_chain_amount);
    Ok(result)
}

pub async fn validate_token_supply_amount(
    reserve_address: &str,
) -> Result<EntryState, Box<dyn std::error::Error>> {
    let calculated_amount = get_token_supply_amount(reserve_address).await?;

    // with the reserve address, look for the reserve token document,
    // then use the aTokenAddress to get the on-chain amount
    let token_data = find_reserve_for_token(reserve_address, ReserveTokenField::Reserve).await?;
    let a_token_address = token_data.aTokenAddress;
    let on_chain_amount = get_total_supply(&a_token_address).await?;

    let result = EntryState::new(calculated_amount, on_chain_amount);
    Ok(result)
}

pub async fn validate_token_borrow_amount(
    reserve_address: &str,
) -> Result<EntryState, Box<dyn std::error::Error>> {
    let calculated_amount = get_token_borrow_amount(reserve_address).await?;

    // with the reserve address, look for the reserve token document,
    // then use the aTokenAddress to get the on-chain amount
    let token_data = find_reserve_for_token(reserve_address, ReserveTokenField::Reserve).await?;
    let v_token_address = token_data.variableDebtTokenAddress;
    let on_chain_amount = get_total_supply(&v_token_address).await?;

    let result = EntryState::new(calculated_amount, on_chain_amount);
    Ok(result)
}
