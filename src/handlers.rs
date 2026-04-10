use crate::output;
use crate::balance_calculator::process_user_token_events;
#[allow(unused_imports)]
use crate::db::{
  find_all_reserves,
  find_reserve_for_token,
  get_orderbook,
  find_all_users,
  get_solver_volume,
  find_docs_with_non_null_timestamp,
  //
  find_all_reserve_addresses,
  find_user_events,
  find_token_events,
  find_user_assets_position,
  find_user_balance_events,
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
use crate::functions::{
  extract_value_from_flags_or_exit, extract_optional_value_from_flags, decimal128_to_u128,
};
use crate::structs::{ReserveTokenField, Flag, FlagType};
use crate::models::{ReserveTokenDocument, SolverVolumeDocument, MoneyMarketEventDocument};
use crate::constants::HELP_MESSAGE;
use futures::future::join_all;
use tokio::task;
use tokio::sync::Semaphore;
use rand::seq::index::sample;
use std::cmp::min;
use std::sync::Arc;

pub async fn handle_help() {
  output!("{}", HELP_MESSAGE);
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
    output!("Orderbook is empty.");
  } else {
    for order in book {
      output!("{:?}", order);
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
    output!("No documents found in the database.");
  } else {
    output!("Total documents in the database: {}", all_docs.len());
  }

  if non_null_docs.is_empty() {
    output!("Coverage: 100% (no documents with null timestamp)");
  } else {
    output!("Documents with non-null timestamp: {}", non_null_docs.len());
  }

  let coverage = if all_docs.is_empty() {
    100.0
  } else {
    (non_null_docs.len() as f64 / all_docs.len() as f64) * 100.0
  };

  output!("Coverage percentage: {:.2}%", coverage);
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
    output!("No reserve tokens found.");
  } else {
    for token in tokens {
      output!("{:?}", token);
    }
  }
}

pub async fn handle_last_block() {
  match get_last_block().await {
    Ok(block) => output!("Latest block number: {}", block),
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
  output!(
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
      output!(
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
      output!("Validating {} timestamp entries...", to_validate);
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
    output!("No valid timestamps found to compare.");
  } else {
    let total_diff: u64 = all_diffs.iter().sum();
    let average_diff = total_diff as f64 / all_diffs.len() as f64;
    let max_diff = all_diffs.iter().max().unwrap_or(&0);
    let min_diff = all_diffs.iter().min().unwrap_or(&0);
    output!(
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

  // Extract optional block number from flags
  let block_number = flags.iter().find_map(|f| match f {
    Flag::Block(block) => Some(*block),
    _ => None,
  });

  match get_balance_of(&token_passed, &user_address, block_number).await {
    Ok(balance) => {
      if let Some(block) = block_number {
        output!(
          "Balance of {} for token {} at block {}: {}",
          user_address, token_passed, block, balance
        );
      } else {
        output!(
          "Balance of {} for token {}: {}",
          user_address, token_passed, balance
        );
      }
    }
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
      Flag::DebtToken(address) => Some((address.clone(), ReserveTokenField::VariableDebtToken)),
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
      output!(
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

pub async fn handle_inspect_user_position(flags: Vec<Flag>) {
  let error_message = "Error: --inspect-user-position requires a user address to be specified.";
  let user_address =
    extract_value_from_flags_or_exit(flags.clone(), FlagType::InspectUserPosition, error_message);

  let token_address_tuple = flags
    .iter()
    .find_map(|f| match f {
      Flag::AToken(address) => Some((address.clone(), ReserveTokenField::AToken)),
      Flag::DebtToken(address) => Some((address.clone(), ReserveTokenField::VariableDebtToken)),
      _ => None,
    })
    .unwrap_or_else(|| {
      eprintln!("Error: --inspect-user-position requires either --a-token or --debt-token");
      std::process::exit(1);
    });

  let (token_address, field) = token_address_tuple;
  let is_a_token = matches!(field, ReserveTokenField::AToken);

  // Get all money market events for the user
  let money_market_events = match find_user_events(&user_address).await {
    Ok(events) => events,
    Err(e) => {
      eprintln!("Error fetching user events: {}", e);
      std::process::exit(1);
    }
  };

  // Get user position from database
  let user_positions = match find_user_assets_position(&user_address).await {
    Ok(positions) => positions,
    Err(e) => {
      eprintln!("Error fetching user position: {}", e);
      std::process::exit(1);
    }
  };

  // Find the specific position for the given token
  let position = user_positions.iter().find(|p| {
    if is_a_token {
      p.aTokenAddress.to_lowercase() == token_address.to_lowercase()
    } else {
      p.variableDebtTokenAddress.to_lowercase() == token_address.to_lowercase()
    }
  });

  let position = match position {
    Some(p) => p,
    None => {
      eprintln!(
        "Error: No position found for user {} with token {}",
        user_address, token_address
      );
      eprintln!("\nAvailable positions for this user:");
      for (idx, p) in user_positions.iter().enumerate() {
        eprintln!("  Position {}:", idx + 1);
        eprintln!("    Reserve: {}", p.reserveAddress);
        eprintln!("    aToken: {}", p.aTokenAddress);
        eprintln!("    debtToken: {}", p.variableDebtTokenAddress);
      }
      std::process::exit(1);
    }
  };

  // Query user_balance_events collection for this user and token type
  let token_type_str = if is_a_token { "aToken" } else { "variableDebtToken" };
  let balance_events = match find_user_balance_events(&user_address, Some(token_type_str)).await {
    Ok(events) => events,
    Err(e) => {
      eprintln!("Error fetching user balance events: {}", e);
      std::process::exit(1);
    }
  };

  // Build set of eventIds from balance events collection
  let mut balance_history_event_ids = std::collections::HashSet::new();
  for event in &balance_events {
    balance_history_event_ids.insert(event.eventId.clone());
  }

  // Also include any legacy embedded eventIds (for old documents pre-migration)
  let legacy_balance_history = if is_a_token {
    &position.aTokenBalanceHistory
  } else {
    &position.debtTokenBalanceHistory
  };
  for entry in legacy_balance_history {
    balance_history_event_ids.insert(entry.eventId.clone());
  }

  // Create eventIds for all money market events and track missing ones
  let mut money_market_event_ids = std::collections::HashSet::new();
  let mut missed_events = Vec::new();

  // Define which event types are relevant for the token type we're inspecting
  let relevant_event_types = if is_a_token {
    vec!["a-token-mint", "a-token-burn", "a-token-transfer"]
  } else {
    vec!["debt-token-mint", "debt-token-burn"]
  };

  let mut relevant_event_count = 0;
  for event in &money_market_events {
    // Skip events that aren't relevant to this token type
    if !relevant_event_types.contains(&event.event_type()) {
      continue;
    }

    // Skip events that aren't for the specific token address we're inspecting
    if let Some(event_token_address) = event.token_address() {
      if event_token_address.to_lowercase() != token_address.to_lowercase() {
        continue;
      }
    } else {
      // Event doesn't have a token address, skip it
      continue;
    }

    relevant_event_count += 1;
    let event_id = format!(
      "{}-{}-{}",
      event.block_number(),
      event.tx_hash(),
      event.log_index()
    );
    money_market_event_ids.insert(event_id.clone());

    if !balance_history_event_ids.contains(&event_id) {
      missed_events.push(serde_json::json!({
        "eventId": event_id,
        "blockNumber": event.block_number(),
        "txHash": event.tx_hash(),
        "logIndex": event.log_index(),
        "eventType": event.event_type(),
      }));
    }
  }

  // Create output JSON
  let output = serde_json::json!({
    "user": user_address,
    "tokenAddress": token_address,
    "tokenType": if is_a_token { "aToken" } else { "debtToken" },
    "eventsOnMoneyMarketEventCollection": relevant_event_count,
    "eventsOnUserBalanceHistory": balance_history_event_ids.len(),
    "eventsMissedCount": missed_events.len(),
    "missedEvents": missed_events,
  });

  output!("{}", serde_json::to_string_pretty(&output).unwrap());
}

pub async fn handle_token(flags: Vec<Flag>) {
  let token_address_tuple = flags
    .iter()
    .find_map(|f| match f {
      Flag::ReserveToken(address) => Some((address.clone(), ReserveTokenField::Reserve)),
      Flag::AToken(address) => Some((address.clone(), ReserveTokenField::AToken)),
      Flag::DebtToken(address) => Some((address.clone(), ReserveTokenField::VariableDebtToken)),
      _ => None,
    })
    .unwrap_or_else(|| {
      eprintln!("Error: --reserve-token, --a-token or --debt-token is required.");
      std::process::exit(1);
    });
  let (token_address, field) = token_address_tuple;

  match find_reserve_for_token(&token_address, field).await {
    Ok(token_data) => output!("Reserve data for token {}: {:?}", token_address, token_data),
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
      output!("User Supply Validation Results:");
      output!("  Database Amount: {}", result.database_amount);
      output!("  On-Chain Amount: {}", result.on_chain_amount);
      output!("  Difference: {}", result.difference);
      output!("  Percentage: {:.4}%", result.percentage);

      let report = compare_and_report_diff(
        result.database_amount,
        result.on_chain_amount,
        &format!(
          "user {} supply for reserve {}",
          user_address, reserve_address
        ),
      );
      output!("  Status: {}", report);
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
      output!("User Scaled Supply Validation Results:");
      output!("  Database Amount: {}", result.database_amount);
      output!("  On-Chain Amount: {}", result.on_chain_amount);
      output!("  Difference: {}", result.difference);
      output!("  Percentage: {:.4}%", result.percentage);

      let report = compare_and_report_diff(
        result.database_amount,
        result.on_chain_amount,
        &format!(
          "user {} scaled supply for reserve {}",
          user_address, reserve_address
        ),
      );
      output!("  Status: {}", report);
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
      output!("User Borrow Validation Results:");
      output!("  Database Amount: {}", result.database_amount);
      output!("  On-Chain Amount: {}", result.on_chain_amount);
      output!("  Difference: {}", result.difference);
      output!("  Percentage: {:.4}%", result.percentage);

      let report = compare_and_report_diff(
        result.database_amount,
        result.on_chain_amount,
        &format!(
          "user {} borrow for reserve {}",
          user_address, reserve_address
        ),
      );
      output!("  Status: {}", report);
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
      output!("User Scaled Borrow Validation Results:");
      output!("  Database Amount: {}", result.database_amount);
      output!("  On-Chain Amount: {}", result.on_chain_amount);
      output!("  Difference: {}", result.difference);
      output!("  Percentage: {:.4}%", result.percentage);

      let report = compare_and_report_diff(
        result.database_amount,
        result.on_chain_amount,
        &format!(
          "user {} scaled borrow for reserve {}",
          user_address, reserve_address
        ),
      );
      output!("  Status: {}", report);
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
      output!("Token Supply Validation Results:");
      output!("  Database Amount: {}", result.database_amount);
      output!("  On-Chain Amount: {}", result.on_chain_amount);
      output!("  Difference: {}", result.difference);
      output!("  Percentage: {:.4}%", result.percentage);

      let report = compare_and_report_diff(
        result.database_amount,
        result.on_chain_amount,
        &format!("total aToken supply for reserve {}", reserve_address),
      );
      output!("  Status: {}", report);
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
      output!("Token Scaled Supply Validation Results:");
      output!("  Database Amount: {}", result.database_amount);
      output!("  On-Chain Amount: {}", result.on_chain_amount);
      output!("  Difference: {}", result.difference);
      output!("  Percentage: {:.4}%", result.percentage);

      let report = compare_and_report_diff(
        result.database_amount,
        result.on_chain_amount,
        &format!("total aToken scaled supply for reserve {}", reserve_address),
      );
      output!("  Status: {}", report);
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
      output!("Token Borrow Validation Results:");
      output!("  Database Amount: {}", result.database_amount);
      output!("  On-Chain Amount: {}", result.on_chain_amount);
      output!("  Difference: {}", result.difference);
      output!("  Percentage: {:.4}%", result.percentage);

      let report = compare_and_report_diff(
        result.database_amount,
        result.on_chain_amount,
        &format!("total debt token supply for reserve {}", reserve_address),
      );
      output!("  Status: {}", report);
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
      output!("Token Scaled Borrow Validation Results:");
      output!("  Database Amount: {}", result.database_amount);
      output!("  On-Chain Amount: {}", result.on_chain_amount);
      output!("  Difference: {}", result.difference);
      output!("  Percentage: {:.4}%", result.percentage);

      let report = compare_and_report_diff(
        result.database_amount,
        result.on_chain_amount,
        &format!(
          "total debt token scaled supply for reserve {}",
          reserve_address
        ),
      );
      output!("  Status: {}", report);
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
  output!("Validating all reserves in parallel...");

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
          output!(
            "❌ Reserve {}: ERROR - {}",
            validation_result.reserve_address, error
          );
        } else {
          output!(
            "✅ Reserve {} validated successfully",
            validation_result.reserve_address
          );
          output!(
            "  Supply - DB: {}\n  On-Chain:    {}\n  Diff: {}, %: {:.6}%",
            validation_result.supply.database_amount,
            validation_result.supply.on_chain_amount,
            validation_result.supply.difference,
            validation_result.supply.percentage
          );
          output!(
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
        output!("❌ Validation failed: {}", e);
      }
      Err(e) => {
        error_count += 1;
        output!("❌ Task failed: {}", e);
      }
    }
  }

  output!(
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
  output!("Validating all users in parallel...");

  // Fetch all users first
  let users = match find_all_users().await {
    Ok(users) => users,
    Err(e) => {
      eprintln!("Error fetching users: {}", e);
      std::process::exit(1);
    }
  };

  // Create a semaphore to limit concurrent user validations
  // This prevents "too many open files" error by limiting concurrent operations
  let max_concurrent_users = 10;
  let semaphore = Arc::new(Semaphore::new(max_concurrent_users));

  // Create tasks for parallel user validation with semaphore
  let tasks: Vec<_> = users
    .into_iter()
    .map(|user| {
      let user_address = user.userAddress.clone();
      let semaphore = Arc::clone(&semaphore);
      task::spawn(async move {
        // Acquire permit before processing
        let _permit = match semaphore.acquire().await {
          Ok(permit) => permit,
          Err(e) => {
            eprintln!(
              "Failed to acquire semaphore permit for user {}: {}",
              user_address, e
            );
            return;
          }
        };

        // Use handle_user_validation instead of calling validate_user_all_positions directly
        if scaled {
          handle_user_validation_scaled(&user_address, false).await;
        } else {
          handle_user_validation(&user_address, false).await;
        }
        // Permit is automatically released when _permit is dropped
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
        output!("❌ Task failed: {}", e);
      }
    }
  }

  output!(
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

  output!("Validating all positions for user {}...", user_address);
  handle_user_validation(&user_address, true).await;
}
pub async fn handle_validate_user_all_scaled(flags: Vec<Flag>) {
  let user_address = extract_value_from_flags_or_exit(
    flags.clone(),
    FlagType::ValidateUserAll,
    "Error: --validate-user-all requires a user address to be specified.",
  );

  output!("Validating all positions for user {}...", user_address);
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
  output!(
    "✅ User {}: {} positions validated",
    result.user_address,
    result.positions.len()
  );
  for position in &result.positions {
    if let Some(error) = &position.error {
      output!(
        "  ❌ Reserve {}: ERROR - {}",
        position.reserve_address, error
      );
    } else {
      output!("  📊 Reserve {}:", position.reserve_address);
      output!(
        "  Supply - DB: {}\n  On-Chain:    {}\n  Diff: {}, %: {:.6}%",
        position.supply.database_amount,
        position.supply.on_chain_amount,
        position.supply.difference,
        position.supply.percentage
      );
      output!(
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
  output!("Validating everything...");

  // Validate all reserves
  output!("\n🔍 Validating all reserves...");
  handle_validate_token_all().await;
  // Validate all users
  output!("\n🔍 Validating all users...");
  handle_validate_users_all().await;

  output!("\n🎉 Complete validation finished!");
}

pub async fn handle_validate_all_scaled() {
  output!("Validating everything...");

  // Validate all reserves
  output!("\n🔍 Validating all reserves...");
  handle_validate_token_all_scaled().await;
  // Validate all users
  output!("\n🔍 Validating all users...");
  handle_validate_users_all_scaled().await;

  output!("\n🎉 Complete validation finished!");
}

// New handlers for the additional CLI features

pub async fn handle_get_all_users() {
  let users = find_all_users().await.unwrap_or_else(|e| {
    eprintln!("Error fetching users: {}", e);
    std::process::exit(1);
  });

  if users.is_empty() {
    output!("No users found.");
  } else {
    output!("All user:");
    for user in &users {
      let count_of_positions = user.positions.len();
      output!(
        "User: {}\n  positions on tokens: {}",
        user.userAddress, count_of_positions
      );
      let mut as_borrower = 0;
      let mut as_supplier = 0;
      for position in &user.positions {
        let borrow = decimal128_to_u128(position.variableDebtTokenBalance);
        let supply = decimal128_to_u128(position.aTokenBalance);
        if borrow > 0 {
          as_borrower += 1;
        }
        if supply > 0 {
          as_supplier += 1;
        }
      }
      output!(
        "  Borrower positions:  {}\n  Supplier positions:  {}\n",
        as_borrower, as_supplier
      );
    }
    output!("Total users: {}", users.len());
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
    output!("No reserves found.");
  } else {
    output!("All reserve tokens:");
    for reserve in &reserves {
      output!(
        "Address: {}, Symbol: {}",
        reserve.reserveAddress, reserve.symbol
      );
    }
    output!("Total reserves: {}", reserves.len());
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
    output!("No aTokens found.");
  } else {
    output!("All aToken addresses:");
    for reserve in &reserves {
      output!(
        "Address: {}, Symbol: {}",
        reserve.aTokenAddress, reserve.symbol
      );
    }
    output!("Total aTokens: {}", reserves.len());
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
    output!("No debt tokens found.");
  } else {
    output!("All debt token addresses:");
    for reserve in &reserves {
      output!(
        "Address: {}, Symbol: {}",
        reserve.variableDebtTokenAddress, reserve.symbol
      );
    }
    output!("Total debt tokens: {}", reserves.len());
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
    output!("No events found for token: {}", token_address);
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
    output!("No events found for user: {}", user_address);
  } else {
    handle_money_market_event_output(events);
  }
}

fn handle_money_market_event_output(event_vector: Vec<MoneyMarketEventDocument>) {
  for event in event_vector {
    match event {
      MoneyMarketEventDocument::ATokenBalanceTransfer(doc) => {
        output!("AToken Balance Transfer Event:");
        output!("  Doc: {:?}", doc);
      }
      MoneyMarketEventDocument::ATokenTransfer(doc) => {
        output!("AToken Transfer Event:");
        output!("  Doc: {:?}", doc);
      }
      MoneyMarketEventDocument::ATokenBurn(doc) => {
        output!("AToken Burn Event:");
        output!("  Doc: {:?}", doc);
      }
      MoneyMarketEventDocument::ATokenMint(doc) => {
        output!("AToken Mint Event:");
        output!("  Doc: {:?}", doc);
      }
      MoneyMarketEventDocument::Borrow(doc) => {
        output!("Borrow Event:");
        output!("  Doc: {:?}", doc);
      }
      MoneyMarketEventDocument::DebtTokenBurn(doc) => {
        output!("Debt Token Burn Event:");
        output!("  Doc: {:?}", doc);
      }
      MoneyMarketEventDocument::DebtTokenMint(doc) => {
        output!("Debt Token Mint Event:");
        output!("  Doc: {:?}", doc);
      }
      MoneyMarketEventDocument::Repay(doc) => {
        output!("Repay Event:");
        output!("  Doc: {:?}", doc);
      }
      MoneyMarketEventDocument::ReserveDataUpdated(doc) => {
        output!("Reserve Data Updated Event:");
        output!("  Doc: {:?}", doc);
      }
      MoneyMarketEventDocument::Supply(doc) => {
        output!("Supply Event:");
        output!("  Doc: {:?}", doc);
      }
      MoneyMarketEventDocument::Withdraw(doc) => {
        output!("Withdraw Event:");
        output!("  Doc: {:?}", doc);
      }
    }
  }
}

async fn handle_validate_reserve_indexes_generic(reserve_address: String) {
  output!("Validating reserve indexes for: {}", reserve_address);
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

  output!("Reserve: {}", reserve_address);
  output!("Token: {}", reserve_data.symbol);
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

  output!("Liquidity Index:");
  output!("  Database: {}", db_liquidity_index);
  output!("  On-Chain: {}", on_chain_liquidity_index);
  output!(
    "  Difference: {}",
    db_liquidity_index.abs_diff(on_chain_liquidity_index)
  );

  output!("Variable Borrow Index:");
  output!("  Database: {}", db_variable_borrow_index);
  output!("  On-Chain: {}", on_chain_variable_borrow_index);
  output!(
    "  Difference: {}",
    db_variable_borrow_index.abs_diff(on_chain_variable_borrow_index)
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
  output!("Validating indexes for all reserves...");

  let reserves = find_all_reserve_addresses().await;

  if reserves.is_empty() {
    output!("No reserves found.");
    return;
  }

  output!("Found {} reserves to validate", reserves.len());

  for reserve in reserves {
    handle_validate_reserve_indexes_generic(reserve).await;
  }

  output!("\n🎉 Reserve index validation complete!");
}

pub async fn handle_calculate_from_events(flags: Vec<Flag>) {
  let user_address = extract_value_from_flags_or_exit(
    flags.clone(),
    FlagType::CalculateFromEvents,
    "Error: --calculate-from-events requires a user address to be specified.",
  );

  // Extract token address from flags (reserve, aToken, or debt token)
  let reserve_token = extract_optional_value_from_flags(&flags, FlagType::ReserveToken);
  let a_token = extract_optional_value_from_flags(&flags, FlagType::AToken);
  let debt_token = extract_optional_value_from_flags(&flags, FlagType::DebtToken);

  // Determine which token address to use and get the reserve address for index lookup
  let (token_address, reserve_address, token_type, is_debt) = if let Some(addr) = reserve_token {
    // Get the aToken address for this reserve
    let reserve_data = match find_reserve_for_token(&addr, ReserveTokenField::Reserve).await {
      Ok(Some(data)) => data,
      Ok(None) => {
        eprintln!("Reserve not found: {}", addr);
        std::process::exit(1);
      }
      Err(e) => {
        eprintln!("Error fetching reserve data: {}", e);
        std::process::exit(1);
      }
    };
    (
      reserve_data.aTokenAddress.clone(),
      reserve_data.reserveAddress,
      "reserve (aToken)",
      false,
    )
  } else if let Some(addr) = a_token {
    // Look up the reserve address for this aToken
    let reserve_data = match find_reserve_for_token(&addr, ReserveTokenField::AToken).await {
      Ok(Some(data)) => data,
      Ok(None) => {
        eprintln!("aToken not found: {}", addr);
        std::process::exit(1);
      }
      Err(e) => {
        eprintln!("Error fetching reserve data for aToken: {}", e);
        std::process::exit(1);
      }
    };
    (addr, reserve_data.reserveAddress, "aToken", false)
  } else if let Some(addr) = debt_token {
    // Look up the reserve address for this debt token
    let reserve_data =
      match find_reserve_for_token(&addr, ReserveTokenField::VariableDebtToken).await {
        Ok(Some(data)) => data,
        Ok(None) => {
          eprintln!("Debt token not found: {}", addr);
          std::process::exit(1);
        }
        Err(e) => {
          eprintln!("Error fetching reserve data for debt token: {}", e);
          std::process::exit(1);
        }
      };
    (addr, reserve_data.reserveAddress, "debt token", true)
  } else {
    eprintln!(
      "Error: --calculate-from-events requires one of: --reserve-token, --a-token, or --debt-token"
    );
    std::process::exit(1);
  };

  output!("\n=== Calculating Scaled Balance from Events ===");
  output!("User Address: {}", user_address);
  output!("Token Type: {}", token_type);
  output!("Token Address: {}", token_address);
  output!("Reserve Address: {}", reserve_address);

  // Fetch all events for the user
  let events = match find_user_events(&user_address).await {
    Ok(events) => events,
    Err(e) => {
      eprintln!("Error fetching user events: {}", e);
      std::process::exit(1);
    }
  };

  if events.is_empty() {
    output!("\nNo events found for user: {}", user_address);
    return;
  }

  output!("Found {} total events for user", events.len());

  // Get the current index using the reserve address (not the token address)
  let current_index = if is_debt {
    match get_variable_borrow_index(&reserve_address).await {
      Ok(index) => index,
      Err(e) => {
        eprintln!("Error fetching variable borrow index: {}", e);
        std::process::exit(1);
      }
    }
  } else {
    match get_atoken_liquidity_index(&reserve_address).await {
      Ok(index) => index,
      Err(e) => {
        eprintln!("Error fetching aToken liquidity index: {}", e);
        std::process::exit(1);
      }
    }
  };

  // Process events and calculate balance
  match process_user_token_events(&events, &user_address, &token_address, current_index) {
    Ok(result) => {
      output!("\n=== Summary ===");
      output!("Scaled Balance: {}", result.scaled_balance);
      output!("Real Balance (from scaled): {}", result.real_balance);
      output!("Last Index Used: {}", result.last_index);
      output!("Current Index: {}", current_index);
      output!("Last Event Block: {}", result.last_event_block);

      // Get on-chain balance at the last event block for accurate comparison
      match get_balance_of(&token_address, &user_address, Some(result.last_event_block)).await {
        Ok(on_chain_balance_at_event) => {
          output!(
            "\n=== On-Chain Comparison (at Last Event Block {}) ===",
            result.last_event_block
          );
          output!("Calculated Balance: {}", result.real_balance);
          output!("On-Chain Balance:   {}", on_chain_balance_at_event);

          let diff = result.real_balance.abs_diff(on_chain_balance_at_event);

          let percentage = if on_chain_balance_at_event == 0 {
            0.0
          } else {
            (diff as f64 / on_chain_balance_at_event as f64) * 100.0
          };

          output!("Difference:         {}", diff);
          output!("Percentage:         {:.4}%", percentage);

          if diff == 0 {
            output!("\n✅ Perfect match!");
          } else if percentage < 0.01 {
            output!("\n✅ Excellent match (< 0.01% difference)");
          } else if percentage < 1.0 {
            output!("\n⚠️  Minor mismatch (< 1% difference)");
          } else {
            output!("\n❌ Significant mismatch (>= 1% difference)");
          }
        }
        Err(e) => {
          eprintln!(
            "\nWarning: Could not fetch on-chain balance at last event block: {}",
            e
          );
        }
      }

      // Also show current on-chain balance for reference
      match get_balance_of(&token_address, &user_address, None).await {
        Ok(current_on_chain_balance) => {
          output!("\n=== Current On-Chain Balance (Latest Block) ===");
          output!("Current Balance:    {}", current_on_chain_balance);

          let diff_current = result.real_balance.abs_diff(current_on_chain_balance);

          if diff_current != 0 {
            output!("Difference:         {}", diff_current);
            output!(
              "Note: This difference is expected if there were events after block {}",
              result.last_event_block
            );
          } else {
            output!(
              "(Matches calculated balance - no events since block {})",
              result.last_event_block
            );
          }
        }
        Err(e) => {
          eprintln!("\nWarning: Could not fetch current on-chain balance: {}", e);
        }
      }
    }
    Err(e) => {
      eprintln!("Error processing events: {}", e);
      std::process::exit(1);
    }
  }
}
