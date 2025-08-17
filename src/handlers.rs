use crate::db::{
    find_all_reserves,
    find_reserve_for_token,
    get_orderbook,
    find_all_users,
    get_solver_volume,
    find_docs_with_non_null_timestamp,
    //
    find_all_user_addresses,
    find_all_reserve_addresses,
    find_user_events,
    find_token_events,
};
use crate::evm::{
    get_last_block, get_balance_of, get_block_timestamp, get_atoken_liquidity_index,
    get_variable_borrow_index,
};
use crate::helpers::{compare_and_report_diff, find_user_scaled_position};
use crate::validators::{
    validate_user_supply_amount, validate_user_borrow_amount, validate_token_supply_amount,
    validate_token_borrow_amount, validate_user_all_positions, validate_user_all_positions_scaled,
    validate_reserve, validate_scaled_reserve, validate_user_scaled_borrow_amount,
    validate_user_scaled_supply_amount, validate_token_scaled_borrow_amount,
    validate_token_scaled_supply_amount,
};
use crate::functions::{extract_value_from_flags_or_exit, extract_optional_value_from_flags};
use crate::structs::{ReserveTokenField, Flag, FlagType};
use crate::models::{ReserveTokenDocument, SolverVolumeDocument, MoneyMarketEventDocument};
use crate::constants::HELP_MESSAGE;
use futures::future::join_all;
use tokio::task;
use rand::seq::index::sample;
use std::cmp::min;

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

async fn handle_compare_timestamp(doc: SolverVolumeDocument) -> Result<u64, String> {
    let timestamp = doc.timestamp;
    #[allow(non_snake_case)]
    let blockNumber = doc.blockNumber;
    let block_timestamp = match get_block_timestamp(blockNumber).await {
        Ok(ts) => ts,
        Err(e) => {
            eprintln!("Error fetching timestamp for block {}: {}", blockNumber, e);
            return Err(format!(
                "Error fetching timestamp for block {}: {}",
                blockNumber, e
            ));
        }
    };
    let timestamp = match timestamp {
        Some(ts) => ts,
        None => {
            eprintln!("Document ID {} has a null timestamp.", doc.id);
            return Err("Document has a null timestamp".to_string());
        }
    };
    let timestamp = timestamp.timestamp_millis() / 1000; // Convert to seconds
    let diff = (block_timestamp as i64) - timestamp;
    println!(
        "Document ID: {}\n Block Number: {}\n Timestamp:       {}\n Block Timestamp: {}\n Diff: {} seconds",
        doc.id, blockNumber, timestamp, block_timestamp, diff
    );
    Ok(diff.unsigned_abs())
}

pub async fn handle_validate_timestamp(flags: Vec<Flag>) {
    // Optional numeric argument: if present, validate only that many entries; otherwise, validate all
    let maybe_count_str = extract_optional_value_from_flags(&flags, FlagType::ValidateTimestamps);

    // create count amount of random indexes that are within the range of all_docs
    let all_docs = match find_docs_with_non_null_timestamp().await {
        Ok(docs) => docs,
        Err(e) => {
            eprintln!("Error fetching documents with non-null timestamp: {}", e);
            std::process::exit(1);
        }
    };

    let docs_to_validate = match maybe_count_str {
        None => {
            // Validate all timestamp entries
            println!(
                "Validating all timestamp entries ({} found)...",
                all_docs.len()
            );
            all_docs
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
            let count = min(count, 100);
            let to_validate = min(count, all_docs.len());
            println!("Validating {} timestamp entries...", to_validate);
            let indexes = sample(&mut rand::rng(), all_docs.len(), to_validate);
            let mut selected_docs = Vec::new();
            for idx in indexes.iter() {
                selected_docs.push(all_docs[idx].clone());
            }
            selected_docs
        }
    };

    // Process documents in parallel using tokio tasks
    let tasks: Vec<_> = docs_to_validate
        .into_iter()
        .map(|doc| task::spawn(async move { handle_compare_timestamp(doc).await }))
        .collect();

    // Wait for all tasks to complete and collect results
    let results = join_all(tasks).await;

    // Process results and collect diffs
    let mut all_diffs: Vec<u64> = Vec::new();
    for result in results {
        match result {
            Ok(Ok(diff)) => all_diffs.push(diff),
            Ok(Err(e)) => {
                eprintln!("Error processing document: {}", e);
            }
            Err(e) => {
                eprintln!("Task join error: {}", e);
            }
        }
    }

    // Print summary of average difference and max difference
    if all_diffs.is_empty() {
        println!("No valid timestamps found to compare.");
    } else {
        let total_diff: u64 = all_diffs.iter().sum();
        let average_diff = total_diff as f64 / all_diffs.len() as f64;
        let max_diff = all_diffs.iter().max().unwrap_or(&0);
        let min_diff = all_diffs.iter().min().unwrap_or(&0);
        println!(
            "Average difference: {:.2} seconds\nMax difference: {} seconds\nMin difference: {} seconds\n (over {} entries)",
            average_diff,
            max_diff,
            min_diff,
            all_diffs.len()
        );
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
            Flag::DebtToken(_) => Some(FlagType::DebtToken),
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
            Flag::DebtToken(address) => {
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

    match find_user_scaled_position(&user_address, &reserve_data.reserveAddress).await {
        Ok(position) => {
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
            Flag::DebtToken(address) => {
                Some((address.clone(), ReserveTokenField::VariableDebtToken))
            }
            _ => None,
        })
        .unwrap_or_else(|| {
            eprintln!("Error: --reserve-token, --a-token or --debt-token is required.");
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

// New handlers for the additional CLI features

pub async fn handle_get_all_users() {
    let users = find_all_user_addresses().await;

    if users.is_empty() {
        println!("No users found.");
    } else {
        println!("All user addresses:");
        for user in &users {
            println!("{}", user);
        }
        println!("Total users: {}", users.len());
    }
}

pub async fn handle_get_all_reserves() {
    let reserves = match find_all_reserves().await {
        Ok(reserves) => reserves,
        Err(e) => {
            eprintln!("Error fetching reserves: {}", e);
            std::process::exit(1);
        }
    };

    if reserves.is_empty() {
        println!("No reserves found.");
    } else {
        println!("All reserve tokens:");
        for reserve in &reserves {
            println!(
                "Address: {}, Symbol: {}",
                reserve.reserveAddress, reserve.symbol
            );
        }
        println!("Total reserves: {}", reserves.len());
    }
}

pub async fn handle_get_all_a_tokens() {
    let reserves = match find_all_reserves().await {
        Ok(reserves) => reserves,
        Err(e) => {
            eprintln!("Error fetching reserves: {}", e);
            std::process::exit(1);
        }
    };

    if reserves.is_empty() {
        println!("No aTokens found.");
    } else {
        println!("All aToken addresses:");
        for reserve in &reserves {
            println!(
                "Address: {}, Symbol: {}",
                reserve.aTokenAddress, reserve.symbol
            );
        }
        println!("Total aTokens: {}", reserves.len());
    }
}

pub async fn handle_get_all_debt_tokens() {
    let reserves = match find_all_reserves().await {
        Ok(reserves) => reserves,
        Err(e) => {
            eprintln!("Error fetching reserves: {}", e);
            std::process::exit(1);
        }
    };

    if reserves.is_empty() {
        println!("No debt tokens found.");
    } else {
        println!("All debt token addresses:");
        for reserve in &reserves {
            println!(
                "Address: {}, Symbol: {}",
                reserve.variableDebtTokenAddress, reserve.symbol
            );
        }
        println!("Total debt tokens: {}", reserves.len());
    }
}

pub async fn handle_get_token_events(flags: Vec<Flag>) {
    let token_address = extract_value_from_flags_or_exit(
        flags.clone(),
        FlagType::GetTokenEvents,
        "Error: --get-token-events requires a token address to be specified.",
    );

    let events = match find_token_events(&token_address).await {
        Ok(events) => events,
        Err(e) => {
            eprintln!("Error fetching token events: {}", e);
            std::process::exit(1);
        }
    };

    if events.is_empty() {
        println!("No events found for token: {}", token_address);
    } else {
        handle_money_market_event_output(events);
    }
}

pub async fn handle_get_user_events(flags: Vec<Flag>) {
    let user_address = extract_value_from_flags_or_exit(
        flags.clone(),
        FlagType::GetUserEvents,
        "Error: --get-user-events requires a user address to be specified.",
    );

    let events = match find_user_events(&user_address).await {
        Ok(events) => events,
        Err(e) => {
            eprintln!("Error fetching user events: {}", e);
            std::process::exit(1);
        }
    };

    if events.is_empty() {
        println!("No events found for user: {}", user_address);
    } else {
        handle_money_market_event_output(events);
    }
}

fn handle_money_market_event_output(event_vector: Vec<MoneyMarketEventDocument>) {
    for event in event_vector {
        match event {
            MoneyMarketEventDocument::ATokenBalanceTransfer(doc) => {
                println!("AToken Balance Transfer Event:");
                println!("  Doc: {:?}", doc);
            }
            MoneyMarketEventDocument::ATokenTransfer(doc) => {
                println!("AToken Transfer Event:");
                println!("  Doc: {:?}", doc);
            }
            MoneyMarketEventDocument::ATokenBurn(doc) => {
                println!("AToken Burn Event:");
                println!("  Doc: {:?}", doc);
            }
            MoneyMarketEventDocument::ATokenMint(doc) => {
                println!("AToken Mint Event:");
                println!("  Doc: {:?}", doc);
            }
            MoneyMarketEventDocument::Borrow(doc) => {
                println!("Borrow Event:");
                println!("  Doc: {:?}", doc);
            }
            MoneyMarketEventDocument::DebtTokenBurn(doc) => {
                println!("Debt Token Burn Event:");
                println!("  Doc: {:?}", doc);
            }
            MoneyMarketEventDocument::DebtTokenMint(doc) => {
                println!("Debt Token Mint Event:");
                println!("  Doc: {:?}", doc);
            }
            MoneyMarketEventDocument::Repay(doc) => {
                println!("Repay Event:");
                println!("  Doc: {:?}", doc);
            }
            MoneyMarketEventDocument::ReserveDataUpdated(doc) => {
                println!("Reserve Data Updated Event:");
                println!("  Doc: {:?}", doc);
            }
            MoneyMarketEventDocument::Supply(doc) => {
                println!("Supply Event:");
                println!("  Doc: {:?}", doc);
            }
            MoneyMarketEventDocument::Withdraw(doc) => {
                println!("Withdraw Event:");
                println!("  Doc: {:?}", doc);
            }
        }
    }
}

async fn handle_validate_reserve_indexes_generic(reserve_address: String) {
    println!("Validating reserve indexes for: {}", reserve_address);
    // Get database values
    let reserve_data =
        match find_reserve_for_token(&reserve_address, ReserveTokenField::Reserve).await {
            Ok(Some(data)) => data,
            Ok(None) => {
                eprintln!("Reserve not found in database: {}", reserve_address);
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("Error fetching reserve data: {}", e);
                std::process::exit(1);
            }
        };

    println!("Reserve: {}", reserve_address);
    println!("Token: {}", reserve_data.symbol);
    // Get on-chain values
    let on_chain_liquidity_index = match get_atoken_liquidity_index(&reserve_address).await {
        Ok(index) => index,
        Err(e) => {
            eprintln!("Error fetching on-chain liquidity index: {}", e);
            std::process::exit(1);
        }
    };

    let on_chain_variable_borrow_index = match get_variable_borrow_index(&reserve_address).await {
        Ok(index) => index,
        Err(e) => {
            eprintln!("Error fetching on-chain variable borrow index: {}", e);
            std::process::exit(1);
        }
    };

    // Convert database values to u128 for comparison
    let db_liquidity_index = reserve_data
        .liquidityIndex
        .to_string()
        .parse::<u128>()
        .unwrap_or(0);
    let db_variable_borrow_index = reserve_data
        .variableBorrowIndex
        .to_string()
        .parse::<u128>()
        .unwrap_or(0);

    println!("Liquidity Index:");
    println!("  Database: {}", db_liquidity_index);
    println!("  On-Chain: {}", on_chain_liquidity_index);
    println!(
        "  Difference: {}",
        if db_liquidity_index > on_chain_liquidity_index {
            db_liquidity_index - on_chain_liquidity_index
        } else {
            on_chain_liquidity_index - db_liquidity_index
        }
    );

    println!("Variable Borrow Index:");
    println!("  Database: {}", db_variable_borrow_index);
    println!("  On-Chain: {}", on_chain_variable_borrow_index);
    println!(
        "  Difference: {}",
        if db_variable_borrow_index > on_chain_variable_borrow_index {
            db_variable_borrow_index - on_chain_variable_borrow_index
        } else {
            on_chain_variable_borrow_index - db_variable_borrow_index
        }
    );
}

pub async fn handle_validate_reserve_indexes(flags: Vec<Flag>) {
    let reserve_address = extract_value_from_flags_or_exit(
        flags.clone(),
        FlagType::ValidateReserveIndexes,
        "Error: --validate-reserve-indexes requires a reserve address to be specified.",
    );

    handle_validate_reserve_indexes_generic(reserve_address).await;
}

pub async fn handle_validate_all_reserve_indexes() {
    println!("Validating indexes for all reserves...");

    let reserves = find_all_reserve_addresses().await;

    if reserves.is_empty() {
        println!("No reserves found.");
        return;
    }

    println!("Found {} reserves to validate", reserves.len());

    for reserve in reserves {
        handle_validate_reserve_indexes_generic(reserve).await;
    }

    println!("\n🎉 Reserve index validation complete!");
}
