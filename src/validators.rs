use crate::db::{find_reserve_for_token, get_user_position, find_all_reserves};
use crate::evm::{get_balance_of, get_scaled_balance_of, get_total_supply, get_scaled_total_supply};
use crate::structs::{
    EntryState, ReserveTokenField, UserPositionValidation, UserEntryState, ReserveEntryState,
};
use crate::helpers::{
    calculate_user_supply_amount, calculate_user_borrow_amount, get_token_scaled_supply_amount,
    get_token_scaled_borrow_amount, calculate_token_supply_amount, calculate_token_borrow_amount,
    find_user_scaled_position,
};
use futures::future::join_all;
// use mongodb::bson::Decimal128;

// fn decimal128_to_u64_blocknumber(d: Decimal128) -> u64 {
//     // Convert Decimal128 to u64
//     d.to_string()
//         .parse::<u64>()
//         .expect("Failed to parse Decimal128 to u64")
// }

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

pub async fn validate_user_scaled_supply_amount(
    user_address: &str,
    reserve_address: &str,
) -> Result<EntryState, Box<dyn std::error::Error>> {
    let scaled_amount = match find_user_scaled_position(user_address, reserve_address).await {
        Ok(position) => {
            let a_token_balance = position
                .aTokenBalance
                .to_string()
                .parse::<u128>()
                .map_err(|_| "Failed to parse variable debt token balance")?;

            Ok::<u128, Box<dyn std::error::Error>>(a_token_balance)
        }
        Err(_) => {
            // User has no position for this reserve, return 0
            Ok(0)
        }
    }
    .unwrap();
    let token_data = find_reserve_for_token(reserve_address, ReserveTokenField::Reserve)
        .await?
        .ok_or("No reserve data found for the specified reserve address")?;

    let a_token_address = token_data.aTokenAddress;
    let on_chain_amount = get_scaled_balance_of(&a_token_address, user_address).await?;

    let result = EntryState::new(scaled_amount, on_chain_amount);
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

pub async fn validate_user_scaled_borrow_amount(
    user_address: &str,
    reserve_address: &str,
) -> Result<EntryState, Box<dyn std::error::Error>> {
    let scaled_amount = match find_user_scaled_position(user_address, reserve_address).await {
        Ok(position) => {
            let a_token_balance = position
                .variableDebtTokenBalance
                .to_string()
                .parse::<u128>()
                .map_err(|_| "Failed to parse variable debt token balance")?;

            Ok::<u128, Box<dyn std::error::Error>>(a_token_balance)
        }
        Err(_) => {
            // User has no position for this reserve, return 0
            Ok(0)
        }
    }
    .unwrap();

    let token_data = find_reserve_for_token(reserve_address, ReserveTokenField::Reserve)
        .await?
        .ok_or("No reserve data found for the specified reserve address")?;

    let variable_debt_token_address = token_data.variableDebtTokenAddress;

    let on_chain_amount = get_scaled_balance_of(&variable_debt_token_address, user_address).await?;

    let result = EntryState::new(scaled_amount, on_chain_amount);
    Ok(result)
}

pub async fn validate_token_scaled_supply_amount(
    reserve_address: &str,
) -> Result<EntryState, Box<dyn std::error::Error>> {
    let calculated_amount = get_token_scaled_supply_amount(reserve_address).await?;

    // with the reserve address, look for the reserve token document,
    // then use the aTokenAddress to get the on-chain amount
    let token_data = find_reserve_for_token(reserve_address, ReserveTokenField::Reserve)
        .await?
        .ok_or("No reserve data found for the specified reserve address")?;
    let a_token_address = token_data.aTokenAddress;
    let on_chain_amount = get_scaled_total_supply(&a_token_address).await?;

    let result = EntryState::new(calculated_amount, on_chain_amount);
    Ok(result)
}
pub async fn validate_token_supply_amount(
    reserve_address: &str,
) -> Result<EntryState, Box<dyn std::error::Error>> {
    let calculated_amount = calculate_token_supply_amount(reserve_address).await?;

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

pub async fn validate_token_scaled_borrow_amount(
    reserve_address: &str,
) -> Result<EntryState, Box<dyn std::error::Error>> {
    let calculated_amount = get_token_scaled_borrow_amount(reserve_address).await?;

    // with the reserve address, look for the reserve token document,
    // then use the aTokenAddress to get the on-chain amount
    let token_data = find_reserve_for_token(reserve_address, ReserveTokenField::Reserve)
        .await?
        .ok_or("No reserve data found for the specified reserve address")?;
    let v_token_address = token_data.variableDebtTokenAddress;
    let on_chain_amount = get_scaled_total_supply(&v_token_address).await?;

    let result = EntryState::new(calculated_amount, on_chain_amount);
    Ok(result)
}

pub async fn validate_token_borrow_amount(
    reserve_address: &str,
) -> Result<EntryState, Box<dyn std::error::Error>> {
    let calculated_amount = calculate_token_borrow_amount(reserve_address).await?;

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

pub async fn validate_user_all_positions_scaled(
    user_address: &str,
) -> Result<UserEntryState, Box<dyn std::error::Error>> {
    validate_user_all_positions_generic(user_address, true).await
}

pub async fn validate_user_all_positions(
    user_address: &str,
) -> Result<UserEntryState, Box<dyn std::error::Error>> {
    validate_user_all_positions_generic(user_address, false).await
}

pub async fn validate_user_all_positions_generic(
    user_address: &str,
    scaled: bool,
) -> Result<UserEntryState, Box<dyn std::error::Error>> {
    let user_positions = get_user_position(user_address).await?;

    let mut results = UserEntryState::new(user_address.to_string());

    // Create tasks for parallel position validation
    let tasks: Vec<_> = user_positions
        .positions
        .into_iter()
        .map(|position| {
            let user_address = user_address.to_string();
            let reserve_address = position.reserveAddress.clone();
            tokio::task::spawn(async move {
                let mut position_validation = UserPositionValidation {
                    reserve_address: reserve_address.clone(),
                    supply: EntryState::new(0, 0),
                    borrow: EntryState::new(0, 0),
                    error: None,
                };
                if scaled {
                    // Validate supply amount
                    match validate_user_scaled_supply_amount(&user_address, &reserve_address).await
                    {
                        Ok(supply_result) => {
                            position_validation.supply = supply_result;
                        }
                        Err(e) => {
                            position_validation.error =
                                Some(format!("Supply validation failed: {}", e));
                            // Continue to try borrow validation
                        }
                    }
                    // Validate borrow amount
                    match validate_user_scaled_borrow_amount(&user_address, &reserve_address).await
                    {
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
                                position_validation.error =
                                    Some(format!("Borrow validation failed: {}", e));
                            }
                            // Continue to try other positions
                        }
                    }
                } else {
                    // Validate supply amount
                    match validate_user_supply_amount(&user_address, &reserve_address).await {
                        Ok(supply_result) => {
                            position_validation.supply = supply_result;
                        }
                        Err(e) => {
                            position_validation.error =
                                Some(format!("Supply validation failed: {}", e));
                            // Continue to try borrow validation
                        }
                    }
                    // Validate borrow amount
                    match validate_user_borrow_amount(&user_address, &reserve_address).await {
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
                                position_validation.error =
                                    Some(format!("Borrow validation failed: {}", e));
                            }
                            // Continue to try other positions
                        }
                    }
                }

                Ok::<UserPositionValidation, Box<dyn std::error::Error + Send + Sync>>(
                    position_validation,
                )
            })
        })
        .collect();

    // Wait for all tasks to complete
    let position_results = join_all(tasks).await;

    // Collect results
    for result in position_results {
        match result {
            Ok(Ok(position_validation)) => {
                results.positions.push(position_validation);
            }
            Ok(Err(e)) => {
                // Handle validation errors
                let error_position = UserPositionValidation {
                    reserve_address: "unknown".to_string(),
                    supply: EntryState::new(0, 0),
                    borrow: EntryState::new(0, 0),
                    error: Some(format!("Position validation failed: {}", e)),
                };
                results.positions.push(error_position);
            }
            Err(e) => {
                // Handle task failures
                let error_position = UserPositionValidation {
                    reserve_address: "unknown".to_string(),
                    supply: EntryState::new(0, 0),
                    borrow: EntryState::new(0, 0),
                    error: Some(format!("Task failed: {}", e)),
                };
                results.positions.push(error_position);
            }
        }
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

pub async fn validate_scaled_reserve(
    reserve_address: &str,
) -> Result<ReserveEntryState, Box<dyn std::error::Error>> {
    let mut results = ReserveEntryState::new(reserve_address.to_string());

    // Validate supply amount
    match validate_token_scaled_supply_amount(reserve_address).await {
        Ok(supply_result) => {
            results.supply = supply_result;
        }
        Err(e) => {
            results.error = Some(format!("Supply validation failed: {}", e));
            // Continue to try borrow validation
        }
    }

    // Validate borrow amount
    match validate_token_scaled_borrow_amount(reserve_address).await {
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

pub async fn validate_all_scaled_reserves()
-> Result<Vec<ReserveEntryState>, Box<dyn std::error::Error>> {
    let reserves = find_all_reserves().await?;
    let mut results = Vec::new();

    for reserve in reserves {
        let reserve_address = reserve.reserveAddress.clone();
        let reserve_results = validate_scaled_reserve(&reserve_address).await?;
        results.push(reserve_results);
    }

    Ok(results)
}
