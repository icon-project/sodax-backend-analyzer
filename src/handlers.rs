use crate::db::{
    find_all_reserves, find_reserve_for_token, get_orderbook, get_user_position, find_all_users,
    get_solver_volume, find_docs_with_non_null_timestamp,
};
use crate::evm::{get_last_block, get_balance_of};
use crate::helpers::compare_and_report_diff;
use crate::validators::{
    validate_user_supply_amount, validate_user_borrow_amount, validate_token_supply_amount,
    validate_token_borrow_amount, validate_user_all_positions, validate_user_all_positions_scaled,
    validate_reserve, validate_scaled_reserve, validate_user_scaled_borrow_amount,
    validate_user_scaled_supply_amount, validate_token_scaled_borrow_amount,
    validate_token_scaled_supply_amount,
};
use crate::functions::{extract_value_from_flags_or_exit, extract_optional_value_from_flags};
use crate::structs::{ReserveTokenField, Flag, FlagType};
use crate::models::ReserveTokenDocument;
use crate::constants::HELP_MESSAGE;
use futures::future::join_all;
use tokio::task;

pub async fn handle_help() {
    println!("{}", HELP_MESSAGE);
}

pub async fn handle_orderbook() {
    let book = match get_orderbook().await {
        Ok(book) => book,
        Err(e) => {
            eprintln!("Error fetching orderbook: {}", e);
            std::process::exit(1);
        }
    };

    if book.is_empty() {
        println!("Orderbook is empty.");
    } else {
        for order in book {
            println!("{:?}", order);
        }
    }
}

pub async fn handle_timestamp_coverage() {
    let all_docs = match get_solver_volume().await {
        Ok(docs) => docs,
        Err(e) => {
            eprintln!("Error fetching solver volume: {}", e);
            std::process::exit(1);
        }
    };

    let non_null_docs = match find_docs_with_non_null_timestamp().await {
        Ok(docs) => docs,
        Err(e) => {
            eprintln!("Error fetching documents with non-null timestamp: {}", e);
            std::process::exit(1);
        }
    };

    if all_docs.is_empty() {
        println!("No documents found in the database.");
    } else {
        println!("Total documents in the database: {}", all_docs.len());
    }

    if non_null_docs.is_empty() {
        println!("Coverage: 100% (no documents with null timestamp)");
    } else {
        println!("Documents with non-null timestamp: {}", non_null_docs.len());
    }

    let coverage = if all_docs.is_empty() {
        100.0
    } else {
        (non_null_docs.len() as f64 / all_docs.len() as f64) * 100.0
    };

    println!("Coverage percentage: {:.2}%", coverage);
}

pub async fn handle_all_tokens() {
    let tokens = match find_all_reserves().await {
        Ok(tokens) => tokens,
        Err(e) => {
            eprintln!("Error fetching reserve tokens: {}", e);
            std::process::exit(1);
        }
    };
    if tokens.is_empty() {
        println!("No reserve tokens found.");
    } else {
        for token in tokens {
            println!("{:?}", token);
        }
    }
}

pub async fn handle_last_block() {
    match get_last_block().await {
        Ok(block) => println!("Latest block number: {}", block),
        Err(e) => {
            eprintln!("Error fetching last block: {}", e);
            std::process::exit(1);
        }
    }
}

pub async fn handle_validate_timestamp(flags: Vec<Flag>) {
    // Optional numeric argument: if present, validate only that many entries; otherwise, validate all
    let maybe_count_str = extract_optional_value_from_flags(&flags, FlagType::ValidateTimestamps);

    match maybe_count_str {
        None => {
            // Validate all timestamp entries
            let all_docs = match find_docs_with_non_null_timestamp().await {
                Ok(docs) => docs,
                Err(e) => {
                    eprintln!("Error fetching documents with non-null timestamp: {}", e);
                    std::process::exit(1);
                }
            };
            println!(
                "Validating all timestamp entries ({} found)...",
                all_docs.len()
            );
            // TODO: add actual validation logic here
        }
        Some(count_str) => {
            let count: usize = match count_str.parse() {
                Ok(n) if n > 0 => n,
                _ => {
                    eprintln!(
                        "Error: --validate-timestamps expects a positive integer (1..=100) when provided an argument."
                    );
                    std::process::exit(1);
                }
            };
            // Cap to a maximum of 100 entries
            let count = std::cmp::min(count, 100);
            let all_docs = match find_docs_with_non_null_timestamp().await {
                Ok(docs) => docs,
                Err(e) => {
                    eprintln!("Error fetching documents with non-null timestamp: {}", e);
                    std::process::exit(1);
                }
            };
            let to_validate = std::cmp::min(count, all_docs.len());
            println!("Validating {} timestamp entries...", to_validate);
            // TODO: add actual validation logic over the first `to_validate` entries
        }
    }
}

pub async fn handle_balance_of(flags: Vec<Flag>) {
    let error_message =
        "Error: --balance-of requires both a token address and a user address to be specified.";

    let token_type_passed = flags
        .iter()
        .find_map(|f| match f {
            Flag::ReserveToken(_) => Some(FlagType::ReserveToken),
            Flag::AToken(_) => Some(FlagType::AToken),
            Flag::VariableToken(_) => Some(FlagType::VariableToken),
            _ => None,
        })
        .unwrap_or_else(|| {
            eprintln!("{}", error_message);
            std::process::exit(1);
        });
    let token_passed =
        extract_value_from_flags_or_exit(flags.clone(), token_type_passed, error_message);

    let user_address =
        extract_value_from_flags_or_exit(flags.clone(), FlagType::BalanceOf, error_message);

    match get_balance_of(&token_passed, &user_address).await {
        Ok(balance) => println!(
            "Balance of {} for token {}: {}",
            user_address, token_passed, balance
        ),
        Err(e) => {
            eprintln!("Error fetching balance: {}", e);
            std::process::exit(1);
        }
    }
}

pub async fn handle_user_position(flags: Vec<Flag>) {
    let error_message = "Error: --user-position requires a user address to be specified.";
    let user_address =
        extract_value_from_flags_or_exit(flags.clone(), FlagType::UserPosition, error_message);

    let token_address_tuple = flags
        .iter()
        .find_map(|f| match f {
            Flag::ReserveToken(address) => Some((address.clone(), ReserveTokenField::Reserve)),
            Flag::AToken(address) => Some((address.clone(), ReserveTokenField::AToken)),
            Flag::VariableToken(address) => {
                Some((address.clone(), ReserveTokenField::VariableDebtToken))
            }
            _ => None,
        })
        .unwrap_or_else(|| {
            eprintln!("{}", error_message);
            std::process::exit(1);
        });
    let (token_address, field) = token_address_tuple;
    let reserve_data: ReserveTokenDocument = find_reserve_for_token(&token_address, field)
        .await
        .unwrap_or_else(|_| {
            eprintln!(
                "Error: No reserve data found for token address {}",
                token_address
            );
            std::process::exit(1);
        })
        .unwrap_or_else(|| {
            eprintln!(
                "Error: No reserve data found for token address {}",
                token_address
            );
            std::process::exit(1);
        });
    match get_user_position(&user_address).await {
        Ok(user_data) => {
            let position = user_data
                .positions
                .iter()
                .find(|position| position.reserveAddress == reserve_data.reserveAddress)
                .unwrap_or_else(|| {
                    eprintln!("Error: No position found for the specified reserve");
                    std::process::exit(1);
                });
            println!(
                "User position for {} on reserve {}: {:?}",
                user_address, token_address, position
            );
        }
        Err(e) => {
            eprintln!("Error fetching user position: {}", e);
            std::process::exit(1);
        }
    }
}

pub async fn handle_token(flags: Vec<Flag>) {
    let token_address_tuple = flags
        .iter()
        .find_map(|f| match f {
            Flag::ReserveToken(address) => Some((address.clone(), ReserveTokenField::Reserve)),
            Flag::AToken(address) => Some((address.clone(), ReserveTokenField::AToken)),
            Flag::VariableToken(address) => {
                Some((address.clone(), ReserveTokenField::VariableDebtToken))
            }
            _ => None,
        })
        .unwrap_or_else(|| {
            eprintln!("Error: --reserve-token, --a-token or --variable-token is required.");
            std::process::exit(1);
        });
    let (token_address, field) = token_address_tuple;

    match find_reserve_for_token(&token_address, field).await {
        Ok(token_data) => println!("Reserve data for token {}: {:?}", token_address, token_data),
        Err(e) => {
            eprintln!("Error fetching reserve token data: {}", e);
            std::process::exit(1);
        }
    }
}

pub async fn handle_validate_user_supply(flags: Vec<Flag>) {
    let user_address = extract_value_from_flags_or_exit(
        flags.clone(),
        FlagType::ValidateUserSupply,
        "Error: --validate-user-supply requires a user address to be specified.",
    );

    let reserve_address = extract_value_from_flags_or_exit(
        flags.clone(),
        FlagType::ReserveToken,
        "Error: --validate-user-supply requires --reserve-token to be specified.",
    );

    match validate_user_supply_amount(&user_address, &reserve_address).await {
        Ok(result) => {
            println!("User Supply Validation Results:");
            println!("  Database Amount: {}", result.database_amount);
            println!("  On-Chain Amount: {}", result.on_chain_amount);
            println!("  Difference: {}", result.difference);
            println!("  Percentage: {:.4}%", result.percentage);

            let report = compare_and_report_diff(
                result.database_amount,
                result.on_chain_amount,
                &format!(
                    "user {} supply for reserve {}",
                    user_address, reserve_address
                ),
            );
            println!("  Status: {}", report);
        }
        Err(e) => {
            eprintln!("Error validating user supply: {}", e);
            std::process::exit(1);
        }
    }
}
pub async fn handle_validate_user_scaled_supply(flags: Vec<Flag>) {
    let user_address = extract_value_from_flags_or_exit(
        flags.clone(),
        FlagType::ValidateUserSupply,
        "Error: --validate-user-supply <USER_ADDRESS> --scaled requires a user address to be specified.",
    );

    let reserve_address = extract_value_from_flags_or_exit(
        flags.clone(),
        FlagType::ReserveToken,
        "Error: --validate-user-supply <USER_ADDRESS> ==scaled requires --reserve-token to be specified.",
    );

    match validate_user_scaled_supply_amount(&user_address, &reserve_address).await {
        Ok(result) => {
            println!("User Scaled Supply Validation Results:");
            println!("  Database Amount: {}", result.database_amount);
            println!("  On-Chain Amount: {}", result.on_chain_amount);
            println!("  Difference: {}", result.difference);
            println!("  Percentage: {:.4}%", result.percentage);

            let report = compare_and_report_diff(
                result.database_amount,
                result.on_chain_amount,
                &format!(
                    "user {} scaled supply for reserve {}",
                    user_address, reserve_address
                ),
            );
            println!("  Status: {}", report);
        }
        Err(e) => {
            eprintln!("Error validating user scaled supply: {}", e);
            std::process::exit(1);
        }
    }
}

pub async fn handle_validate_user_borrow(flags: Vec<Flag>) {
    let user_address = extract_value_from_flags_or_exit(
        flags.clone(),
        FlagType::ValidateUserBorrow,
        "Error: --validate-user-borrow requires a user address to be specified.",
    );

    let reserve_address = extract_value_from_flags_or_exit(
        flags.clone(),
        FlagType::ReserveToken,
        "Error: --validate-user-borrow requires --reserve-token to be specified.",
    );

    match validate_user_borrow_amount(&user_address, &reserve_address).await {
        Ok(result) => {
            println!("User Borrow Validation Results:");
            println!("  Database Amount: {}", result.database_amount);
            println!("  On-Chain Amount: {}", result.on_chain_amount);
            println!("  Difference: {}", result.difference);
            println!("  Percentage: {:.4}%", result.percentage);

            let report = compare_and_report_diff(
                result.database_amount,
                result.on_chain_amount,
                &format!(
                    "user {} borrow for reserve {}",
                    user_address, reserve_address
                ),
            );
            println!("  Status: {}", report);
        }
        Err(e) => {
            eprintln!("Error validating user borrow: {}", e);
            std::process::exit(1);
        }
    }
}

pub async fn handle_validate_user_scaled_borrow(flags: Vec<Flag>) {
    let user_address = extract_value_from_flags_or_exit(
        flags.clone(),
        FlagType::ValidateUserBorrow,
        "Error: --validate-user-borrow <USER_ADDRESS> --scaled requires a user address to be specified.",
    );

    let reserve_address = extract_value_from_flags_or_exit(
        flags.clone(),
        FlagType::ReserveToken,
        "Error: --validate-user-borrow <USER_ADDRESS> --scaled requires --reserve-token to be specified.",
    );

    match validate_user_scaled_borrow_amount(&user_address, &reserve_address).await {
        Ok(result) => {
            println!("User Scaled Borrow Validation Results:");
            println!("  Database Amount: {}", result.database_amount);
            println!("  On-Chain Amount: {}", result.on_chain_amount);
            println!("  Difference: {}", result.difference);
            println!("  Percentage: {:.4}%", result.percentage);

            let report = compare_and_report_diff(
                result.database_amount,
                result.on_chain_amount,
                &format!(
                    "user {} scaled borrow for reserve {}",
                    user_address, reserve_address
                ),
            );
            println!("  Status: {}", report);
        }
        Err(e) => {
            eprintln!("Error validating user scaled borrow: {}", e);
            std::process::exit(1);
        }
    }
}

pub async fn handle_validate_token_supply(flags: Vec<Flag>) {
    let reserve_address = extract_value_from_flags_or_exit(
        flags.clone(),
        FlagType::ReserveToken,
        "Error: --validate-token-supply requires --reserve-token to be specified.",
    );

    match validate_token_supply_amount(&reserve_address).await {
        Ok(result) => {
            println!("Token Supply Validation Results:");
            println!("  Database Amount: {}", result.database_amount);
            println!("  On-Chain Amount: {}", result.on_chain_amount);
            println!("  Difference: {}", result.difference);
            println!("  Percentage: {:.4}%", result.percentage);

            let report = compare_and_report_diff(
                result.database_amount,
                result.on_chain_amount,
                &format!("total aToken supply for reserve {}", reserve_address),
            );
            println!("  Status: {}", report);
        }
        Err(e) => {
            eprintln!("Error validating token supply: {}", e);
            std::process::exit(1);
        }
    }
}

pub async fn handle_validate_token_scaled_supply(flags: Vec<Flag>) {
    let reserve_address = extract_value_from_flags_or_exit(
        flags.clone(),
        FlagType::ReserveToken,
        "Error: --validate-token-supply --scaled requires --reserve-token to be specified.",
    );

    match validate_token_scaled_supply_amount(&reserve_address).await {
        Ok(result) => {
            println!("Token Scaled Supply Validation Results:");
            println!("  Database Amount: {}", result.database_amount);
            println!("  On-Chain Amount: {}", result.on_chain_amount);
            println!("  Difference: {}", result.difference);
            println!("  Percentage: {:.4}%", result.percentage);

            let report = compare_and_report_diff(
                result.database_amount,
                result.on_chain_amount,
                &format!("total aToken scaled supply for reserve {}", reserve_address),
            );
            println!("  Status: {}", report);
        }
        Err(e) => {
            eprintln!("Error validating scaled token supply: {}", e);
            std::process::exit(1);
        }
    }
}
pub async fn handle_validate_token_borrow(flags: Vec<Flag>) {
    let reserve_address = extract_value_from_flags_or_exit(
        flags.clone(),
        FlagType::ReserveToken,
        "Error: --validate-token-borrow requires --reserve-token to be specified.",
    );

    match validate_token_borrow_amount(&reserve_address).await {
        Ok(result) => {
            println!("Token Borrow Validation Results:");
            println!("  Database Amount: {}", result.database_amount);
            println!("  On-Chain Amount: {}", result.on_chain_amount);
            println!("  Difference: {}", result.difference);
            println!("  Percentage: {:.4}%", result.percentage);

            let report = compare_and_report_diff(
                result.database_amount,
                result.on_chain_amount,
                &format!("total debt token supply for reserve {}", reserve_address),
            );
            println!("  Status: {}", report);
        }
        Err(e) => {
            eprintln!("Error validating token borrow: {}", e);
            std::process::exit(1);
        }
    }
}

pub async fn handle_validate_token_scaled_borrow(flags: Vec<Flag>) {
    let reserve_address = extract_value_from_flags_or_exit(
        flags.clone(),
        FlagType::ReserveToken,
        "Error: --validate-token-borrow --scaled requires --reserve-token to be specified.",
    );

    match validate_token_scaled_borrow_amount(&reserve_address).await {
        Ok(result) => {
            println!("Token Scaled Borrow Validation Results:");
            println!("  Database Amount: {}", result.database_amount);
            println!("  On-Chain Amount: {}", result.on_chain_amount);
            println!("  Difference: {}", result.difference);
            println!("  Percentage: {:.4}%", result.percentage);

            let report = compare_and_report_diff(
                result.database_amount,
                result.on_chain_amount,
                &format!(
                    "total debt token scaled supply for reserve {}",
                    reserve_address
                ),
            );
            println!("  Status: {}", report);
        }
        Err(e) => {
            eprintln!("Error validating token scaled borrow: {}", e);
            std::process::exit(1);
        }
    }
}

pub async fn handle_validate_token_all() {
    handle_validate_token_all_generic(false).await;
}

pub async fn handle_validate_token_all_scaled() {
    handle_validate_token_all_generic(true).await;
}

pub async fn handle_validate_token_all_generic(scaled: bool) {
    println!("Validating all reserves in parallel...");

    // Get all reserves first
    let reserves = match find_all_reserves().await {
        Ok(reserves) => reserves,
        Err(e) => {
            eprintln!("Error fetching reserve tokens: {}", e);
            std::process::exit(1);
        }
    };

    // Create tasks for parallel validation
    let tasks: Vec<_> = reserves
        .into_iter()
        .map(|reserve| {
            let reserve_address = reserve.reserveAddress.clone();
            task::spawn(async move {
                if scaled {
                    match validate_scaled_reserve(&reserve_address).await {
                        Ok(result) => Ok(result),
                        Err(e) => Err(format!("Failed to validate {}: {}", reserve_address, e)),
                    }
                } else {
                    match validate_reserve(&reserve_address).await {
                        Ok(result) => Ok(result),
                        Err(e) => Err(format!("Failed to validate {}: {}", reserve_address, e)),
                    }
                }
            })
        })
        .collect();

    // Wait for all tasks to complete
    let results = join_all(tasks).await;

    let mut success_count = 0;
    let mut error_count = 0;

    for result in results {
        match result {
            Ok(Ok(validation_result)) => {
                success_count += 1;
                if let Some(error) = &validation_result.error {
                    error_count += 1;
                    println!(
                        "❌ Reserve {}: ERROR - {}",
                        validation_result.reserve_address, error
                    );
                } else {
                    println!(
                        "✅ Reserve {} validated successfully",
                        validation_result.reserve_address
                    );
                    println!(
                        "  Supply - DB: {}\n  On-Chain:    {}\n  Diff: {}, %: {:.6}%",
                        validation_result.supply.database_amount,
                        validation_result.supply.on_chain_amount,
                        validation_result.supply.difference,
                        validation_result.supply.percentage
                    );
                    println!(
                        "  Borrow - DB: {}\n  On-Chain:    {}\n  Diff: {}, %: {:.6}%",
                        validation_result.borrow.database_amount,
                        validation_result.borrow.on_chain_amount,
                        validation_result.borrow.difference,
                        validation_result.borrow.percentage
                    );
                }
            }
            Ok(Err(e)) => {
                error_count += 1;
                println!("❌ Validation failed: {}", e);
            }
            Err(e) => {
                error_count += 1;
                println!("❌ Task failed: {}", e);
            }
        }
    }

    println!(
        "\n📊 Summary: {} successful, {} errors",
        success_count, error_count
    );
}

pub async fn handle_validate_users_all() {
    handle_validate_users_all_generic(false).await;
}

pub async fn handle_validate_users_all_scaled() {
    handle_validate_users_all_generic(true).await;
}

pub async fn handle_validate_users_all_generic(scaled: bool) {
    println!("Validating all users in parallel...");

    // Fetch all users first
    let users = match find_all_users().await {
        Ok(users) => users,
        Err(e) => {
            eprintln!("Error fetching users: {}", e);
            std::process::exit(1);
        }
    };

    // Create tasks for parallel user validation
    let tasks: Vec<_> = users
        .into_iter()
        .map(|user| {
            let user_address = user.userAddress.clone();
            task::spawn(async move {
                // Use handle_user_validation instead of calling validate_user_all_positions directly
                if scaled {
                    handle_user_validation_scaled(&user_address, false).await;
                } else {
                    handle_user_validation(&user_address, false).await;
                }
            })
        })
        .collect();

    // Wait for all tasks to complete
    let results = join_all(tasks).await;

    let mut success_count = 0;
    let mut error_count = 0;

    for result in results {
        match result {
            Ok(_) => {
                success_count += 1;
            }
            Err(e) => {
                error_count += 1;
                println!("❌ Task failed: {}", e);
            }
        }
    }

    println!(
        "\n📊 Summary: {} successful users, {} errors",
        success_count, error_count
    );
}

pub async fn handle_validate_user_all(flags: Vec<Flag>) {
    let user_address = extract_value_from_flags_or_exit(
        flags.clone(),
        FlagType::ValidateUserAll,
        "Error: --validate-user-all requires a user address to be specified.",
    );

    println!("Validating all positions for user {}...", user_address);
    handle_user_validation(&user_address, true).await;
}
pub async fn handle_validate_user_all_scaled(flags: Vec<Flag>) {
    let user_address = extract_value_from_flags_or_exit(
        flags.clone(),
        FlagType::ValidateUserAll,
        "Error: --validate-user-all requires a user address to be specified.",
    );

    println!("Validating all positions for user {}...", user_address);
    handle_user_validation_scaled(&user_address, true).await;
}

async fn handle_user_validation(user_address: &str, exit_on_error: bool) {
    handle_user_validation_generic(user_address, exit_on_error, false).await;
}

async fn handle_user_validation_scaled(user_address: &str, exit_on_error: bool) {
    handle_user_validation_generic(user_address, exit_on_error, true).await;
}

async fn handle_user_validation_generic(user_address: &str, exit_on_error: bool, scaled: bool) {
    let result = if scaled {
        match validate_user_all_positions_scaled(user_address).await {
            Ok(result_inner) => result_inner,
            Err(e) => {
                eprintln!("Error validating user {}: {}", user_address, e);
                if exit_on_error {
                    std::process::exit(1);
                }
                return;
            }
        }
    } else {
        match validate_user_all_positions(user_address).await {
            Ok(result_inner) => result_inner,
            Err(e) => {
                eprintln!("Error validating user {}: {}", user_address, e);
                if exit_on_error {
                    std::process::exit(1);
                }
                return;
            }
        }
    };
    println!(
        "✅ User {}: {} positions validated",
        result.user_address,
        result.positions.len()
    );
    for position in &result.positions {
        if let Some(error) = &position.error {
            println!(
                "  ❌ Reserve {}: ERROR - {}",
                position.reserve_address, error
            );
        } else {
            println!("  📊 Reserve {}:", position.reserve_address);
            println!(
                "  Supply - DB: {}\n  On-Chain:    {}\n  Diff: {}, %: {:.6}%",
                position.supply.database_amount,
                position.supply.on_chain_amount,
                position.supply.difference,
                position.supply.percentage
            );
            println!(
                "  Supply - DB: {}\n  On-Chain:    {}\n  Diff: {}, %: {:.6}%",
                position.borrow.database_amount,
                position.borrow.on_chain_amount,
                position.borrow.difference,
                position.borrow.percentage
            );
        }
    }
}

pub async fn handle_validate_all() {
    println!("Validating everything...");

    // Validate all reserves
    println!("\n🔍 Validating all reserves...");
    handle_validate_token_all().await;
    // Validate all users
    println!("\n🔍 Validating all users...");
    handle_validate_users_all().await;

    println!("\n🎉 Complete validation finished!");
}

pub async fn handle_validate_all_scaled() {
    println!("Validating everything...");

    // Validate all reserves
    println!("\n🔍 Validating all reserves...");
    handle_validate_token_all_scaled().await;
    // Validate all users
    println!("\n🔍 Validating all users...");
    handle_validate_users_all_scaled().await;

    println!("\n🎉 Complete validation finished!");
}
