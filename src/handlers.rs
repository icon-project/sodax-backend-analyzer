use crate::cli::Flag;
use crate::db::{
    find_all_reserves, find_reserve_for_token, get_orderbook, get_user_position, ReserveTokenField,
};
use crate::evm::{get_last_block, get_balance_of};
use crate::validation::{
    compare_and_report_diff, validate_user_supply_amount, validate_user_borrow_amount,
    validate_token_supply_amount, validate_token_borrow_amount,
};
use crate::models::ReserveTokenDocument;
use crate::cli::HELP_MESSAGE;

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

pub async fn handle_balance_of(flags: Vec<Flag>) {
    let token_passed = flags
        .iter()
        .find_map(|f| match f {
            Flag::ReserveToken(address) => Some(address.clone()),
            Flag::AToken(address) => Some(address.clone()),
            Flag::VariableToken(address) => Some(address.clone()),
            _ => None,
        })
        .unwrap_or_else(|| {
            eprintln!("Error: --balance-of requires a token address to be specified.");
            std::process::exit(1);
        });

    let user_address = flags
        .iter()
        .find_map(|f| {
            if let Flag::BalanceOf(address) = f {
                Some(address.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            eprintln!("Error: --balance-of requires a user address to be specified.");
            std::process::exit(1);
        });

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
    let user_address = flags
        .iter()
        .find_map(|f| {
            if let Flag::UserPosition(address) = f {
                Some(address.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            eprintln!("Error: --user-position requires a user address to be specified.");
            std::process::exit(1);
        });

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
            eprintln!("Error: --user-position requires a token address to be specified.");
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
        });
    match get_user_position(&user_address).await {
        Ok(Some(user_data)) => {
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
        Ok(None) => {
            eprintln!("Error: No user position found for user {}", user_address);
            std::process::exit(1);
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
    let user_address = flags
        .iter()
        .find_map(|f| {
            if let Flag::ValidateUserSupply(address) = f {
                Some(address.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            eprintln!("Error: --validate-user-supply requires a user address to be specified.");
            std::process::exit(1);
        });

    let reserve_address = flags
        .iter()
        .find_map(|f| {
            if let Flag::ReserveToken(address) = f {
                Some(address.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            eprintln!("Error: --validate-user-supply requires --reserve-token to be specified.");
            std::process::exit(1);
        });

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

pub async fn handle_validate_user_borrow(flags: Vec<Flag>) {
    let user_address = flags
        .iter()
        .find_map(|f| {
            if let Flag::ValidateUserBorrow(address) = f {
                Some(address.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            eprintln!("Error: --validate-user-borrow requires a user address to be specified.");
            std::process::exit(1);
        });

    let reserve_address = flags
        .iter()
        .find_map(|f| {
            if let Flag::ReserveToken(address) = f {
                Some(address.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            eprintln!("Error: --validate-user-borrow requires --reserve-token to be specified.");
            std::process::exit(1);
        });

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

pub async fn handle_validate_token_supply(flags: Vec<Flag>) {
    let reserve_address = flags
        .iter()
        .find_map(|f| {
            if let Flag::ReserveToken(address) = f {
                Some(address.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            eprintln!("Error: --validate-token-supply requires --reserve-token to be specified.");
            std::process::exit(1);
        });

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

pub async fn handle_validate_token_borrow(flags: Vec<Flag>) {
    let reserve_address = flags
        .iter()
        .find_map(|f| {
            if let Flag::ReserveToken(address) = f {
                Some(address.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            eprintln!("Error: --validate-token-borrow requires --reserve-token to be specified.");
            std::process::exit(1);
        });

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
