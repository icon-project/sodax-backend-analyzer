use crate::constants::RAY;
use crate::db::{find_reserve_for_token, get_user_position, ReserveTokenField, find_all_reserves};
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

        // Handle division by zero and edge cases
        let percentage = if on_chain_amount == 0 {
            if database_amount == 0 {
                0.0 // Both are 0, so 0% difference
            } else {
                100.0 // Database has amount but on-chain is 0, so 100% difference
            }
        } else if database_amount == 0 {
            100.0 // Database is 0 but on-chain has amount, so 100% difference
        } else {
            (difference as f64 / on_chain_amount as f64) * 100.0
        };

        EntryState {
            database_amount,
            on_chain_amount,
            difference,
            percentage,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserPositionValidation {
    pub reserve_address: String,
    pub supply: EntryState,
    pub borrow: EntryState,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UserEntryState {
    pub user_address: String,
    pub positions: Vec<UserPositionValidation>,
}

impl UserEntryState {
    pub fn new(user_address: String) -> Self {
        UserEntryState {
            user_address,
            positions: Vec::new(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct ReserveEntryState {
    pub reserve_address: String,
    pub supply: EntryState,
    pub borrow: EntryState,
    pub error: Option<String>,
}

impl ReserveEntryState {
    pub fn new(reserve_address: String) -> Self {
        ReserveEntryState {
            reserve_address,
            supply: EntryState::new(0, 0),
            borrow: EntryState::new(0, 0),
            error: None,
        }
    }

    pub fn with_error(reserve_address: String, error: String) -> Self {
        ReserveEntryState {
            reserve_address,
            supply: EntryState::new(0, 0),
            borrow: EntryState::new(0, 0),
            error: Some(error),
        }
    }
}

async fn find_user_position(
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
    match find_user_position(user_address, reserve_address).await {
        Ok(position) => {
            // Convert Decimal128 to u128 for calculation
            let a_token_balance = position
                .aTokenBalance
                .to_string()
                .parse::<u128>()
                .map_err(|_| "Failed to parse aToken balance")?;

            let real_balance = calculate_real_balance(a_token_balance, index);
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
    match find_user_position(user_address, reserve_address).await {
        Ok(position) => {
            // Convert Decimal128 to u128 for calculation
            let a_token_balance = position
                .variableDebtTokenBalance
                .to_string()
                .parse::<u128>()
                .map_err(|_| "Failed to parse variable debt token balance")?;

            let real_balance = calculate_real_balance(a_token_balance, index);
            Ok(real_balance)
        }
        Err(_) => {
            // User has no position for this reserve, return 0
            Ok(0)
        }
    }
}

pub async fn get_token_supply_amount(
    reserve_address: &str,
) -> Result<u128, Box<dyn std::error::Error>> {
    let index = get_atoken_liquidity_index(reserve_address).await?;

    let token_data = find_reserve_for_token(reserve_address, ReserveTokenField::Reserve)
        .await?
        .ok_or("No reserve data found for the specified reserve address")?;

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

    let token_data = find_reserve_for_token(reserve_address, ReserveTokenField::Reserve)
        .await?
        .ok_or("No reserve data found for the specified reserve address")?;

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

    let token_data = find_reserve_for_token(reserve_address, ReserveTokenField::Reserve)
        .await?
        .ok_or("No reserve data found for the specified reserve address")?;

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

    let token_data = find_reserve_for_token(reserve_address, ReserveTokenField::Reserve)
        .await?
        .ok_or("No reserve data found for the specified reserve address")?;

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
    let token_data = find_reserve_for_token(reserve_address, ReserveTokenField::Reserve)
        .await?
        .ok_or("No reserve data found for the specified reserve address")?;
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
    let token_data = find_reserve_for_token(reserve_address, ReserveTokenField::Reserve)
        .await?
        .ok_or("No reserve data found for the specified reserve address")?;
    let v_token_address = token_data.variableDebtTokenAddress;
    let on_chain_amount = get_total_supply(&v_token_address).await?;

    let result = EntryState::new(calculated_amount, on_chain_amount);
    Ok(result)
}

pub async fn validate_user_all_positions(
    user_address: &str,
) -> Result<UserEntryState, Box<dyn std::error::Error>> {
    let user_positions = get_user_position(user_address).await?;

    let mut results = UserEntryState::new(user_address.to_string());

    for position in user_positions.positions {
        let reserve_address = position.reserveAddress.clone();
        let mut position_validation = UserPositionValidation {
            reserve_address: reserve_address.clone(),
            supply: EntryState::new(0, 0),
            borrow: EntryState::new(0, 0),
            error: None,
        };

        // Validate supply amount
        match validate_user_supply_amount(user_address, &reserve_address).await {
            Ok(supply_result) => {
                position_validation.supply = supply_result;
            }
            Err(e) => {
                position_validation.error = Some(format!("Supply validation failed: {}", e));
                // Continue to try borrow validation
            }
        }

        // Validate borrow amount
        match validate_user_borrow_amount(user_address, &reserve_address).await {
            Ok(borrow_result) => {
                position_validation.borrow = borrow_result;
            }
            Err(e) => {
                // If there's already an error, append to it, otherwise create new error
                if let Some(existing_error) = &position_validation.error {
                    position_validation.error = Some(format!(
                        "{}; Borrow validation failed: {}",
                        existing_error, e
                    ));
                } else {
                    position_validation.error = Some(format!("Borrow validation failed: {}", e));
                }
                // Continue to try other positions
            }
        }

        // Add this position's validation to results
        results.positions.push(position_validation);
    }

    Ok(results)
}

pub async fn validate_reserve(
    reserve_address: &str,
) -> Result<ReserveEntryState, Box<dyn std::error::Error>> {
    let mut results = ReserveEntryState::new(reserve_address.to_string());

    // Validate supply amount
    match validate_token_supply_amount(reserve_address).await {
        Ok(supply_result) => {
            results.supply = supply_result;
        }
        Err(e) => {
            results.error = Some(format!("Supply validation failed: {}", e));
            // Continue to try borrow validation
        }
    }

    // Validate borrow amount
    match validate_token_borrow_amount(reserve_address).await {
        Ok(borrow_result) => {
            results.borrow = borrow_result;
        }
        Err(e) => {
            results.error = Some(format!("Borrow validation failed: {}", e));
            // Both validations attempted, return with any errors
        }
    }

    Ok(results)
}

pub async fn validate_all_reserves() -> Result<Vec<ReserveEntryState>, Box<dyn std::error::Error>> {
    let reserves = find_all_reserves().await?;
    let mut results = Vec::new();

    for reserve in reserves {
        let reserve_address = reserve.reserveAddress.clone();
        let reserve_results = validate_reserve(&reserve_address).await?;
        results.push(reserve_results);
    }

    Ok(results)
}
