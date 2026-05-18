use crate::output;
use crate::balance_calculator::{process_user_token_events, should_skip_transfer_event};
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
  find_token_events_sorted,
  find_user_assets_position,
  get_user_position,
  find_user_balance_events,
  get_partner_asset,
  find_partner_asset_for_receiver,
};
use crate::evm::{
  get_last_block, get_balance_of, get_block_timestamp, get_atoken_liquidity_index,
  get_variable_borrow_index, get_scaled_balance_of,
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
use crate::structs::{ReserveTokenField, Flag, FlagType, ThreeWayComparison, EventValidationResult};
use crate::models::{
  ReserveTokenDocument, SolverVolumeDocument, MoneyMarketEventDocument, PartnerAssetDocument,
  PartnerOutput, UserAssetPositionDocument,
};
use crate::intent_data_decoder::{extract_fee_from_intent_data, FeeIntentData};
use crate::constants::{HELP_MESSAGE, RAY};
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

/// Reconstructs the eventId in the format the sodax-backend writes to
/// `user_balance_events`. For `a-token-transfer` events where the inspected
/// user is the sender, the backend appends a `-from` suffix; every other
/// relevant event uses the plain `<block>-<tx>-<logIndex>` form.
fn reconstruct_balance_event_id(
  block_number: u64,
  tx_hash: &str,
  log_index: i64,
  transfer_from: Option<&str>,
  user_address: &str,
) -> String {
  let is_outgoing_transfer = transfer_from
    .map(|from| from.eq_ignore_ascii_case(user_address))
    .unwrap_or(false);
  let suffix = if is_outgoing_transfer { "-from" } else { "" };
  format!("{}-{}-{}{}", block_number, tx_hash, log_index, suffix)
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
  let balance_events = match find_user_balance_events(&user_address, &position.reserveAddress, Some(token_type_str)).await {
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

    // Skip a-token-transfer events involving the zero address — the sodax-backend
    // does not record them in user balance history (they correspond to mint/burn).
    if should_skip_transfer_event(event) {
      continue;
    }

    relevant_event_count += 1;

    let event_id = reconstruct_balance_event_id(
      event.block_number(),
      event.tx_hash(),
      event.log_index(),
      event.transfer_from(),
      &user_address,
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
  match process_user_token_events(&events, &user_address, &token_address, current_index, true) {
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

// ============================================================
// --validate-from-events / --validate-from-events-all handlers
// ============================================================

pub async fn handle_validate_from_events(flags: Vec<Flag>) {
  let user_address = extract_value_from_flags_or_exit(
    flags.clone(),
    FlagType::ValidateFromEvents,
    "Error: --validate-from-events requires a user address to be specified.",
  );

  let reserve_filter = extract_optional_value_from_flags(&flags, FlagType::ReserveToken);

  println!("\n=== Event Replay Validation ===");
  println!("User: {}", user_address);
  if let Some(ref reserve) = reserve_filter {
    println!("Reserve filter: {}", reserve);
  }

  // Get user positions from DB
  let user_position = match get_user_position(&user_address).await {
    Ok(pos) => pos,
    Err(e) => {
      eprintln!("Error fetching user position: {}", e);
      std::process::exit(1);
    }
  };

  if user_position.positions.is_empty() {
    println!("\nNo positions found for user: {}", user_address);
    return;
  }

  // Filter positions if --reserve-token was provided
  let positions: Vec<_> = if let Some(ref reserve) = reserve_filter {
    let reserve_lower = reserve.to_lowercase();
    user_position
      .positions
      .into_iter()
      .filter(|p| p.reserveAddress.to_lowercase() == reserve_lower)
      .collect()
  } else {
    user_position.positions
  };

  if positions.is_empty() {
    println!(
      "\nNo position found for reserve: {}",
      reserve_filter.unwrap_or_default()
    );
    return;
  }

  println!("Positions to validate: {}\n", positions.len());

  for position in &positions {
    let result = validate_position_from_events(&user_address, position, true).await;
    print_event_validation_result(&result);
  }

  println!("\n=== Event Replay Validation Complete ===");
}

pub async fn handle_validate_from_events_all() {
  println!("\n=== Event Replay Validation (All Users) ===");

  // Fetch all users
  let users = match find_all_users().await {
    Ok(users) => users,
    Err(e) => {
      eprintln!("Error fetching users: {}", e);
      std::process::exit(1);
    }
  };

  let total_users = users.len();
  println!("Total users to validate: {}\n", total_users);

  let semaphore = Arc::new(Semaphore::new(10));

  let tasks: Vec<_> = users
    .into_iter()
    .enumerate()
    .map(|(idx, user)| {
      let user_address = user.userAddress.clone();
      let semaphore = Arc::clone(&semaphore);
      let total = total_users;
      task::spawn(async move {
        let _permit = match semaphore.acquire().await {
          Ok(permit) => permit,
          Err(e) => {
            eprintln!(
              "Failed to acquire semaphore permit for user {}: {}",
              user_address, e
            );
            return Vec::new();
          }
        };

        println!("[{}/{}] Validating user {}...", idx + 1, total, user_address);

        let user_position = match get_user_position(&user_address).await {
          Ok(pos) => pos,
          Err(e) => {
            eprintln!("Error fetching position for user {}: {}", user_address, e);
            return Vec::new();
          }
        };

        let mut results = Vec::new();
        for position in &user_position.positions {
          let result = validate_position_from_events(&user_address, position, false).await;
          results.push(result);
        }
        results
      })
    })
    .collect();

  let all_results = join_all(tasks).await;

  // Aggregate results
  let mut total_positions = 0u64;
  let mut no_events_count = 0u64;
  let mut events_vs_chain_mismatches = 0u64;
  let mut db_vs_chain_mismatches = 0u64;
  let mut events_vs_db_mismatches = 0u64;
  let mut error_count = 0u64;
  // (user, reserve, side, pct, from_events, from_db, on_chain)
  let mut worst_offenders: Vec<(String, String, String, f64, u128, u128, u128)> = Vec::new();

  for task_result in all_results {
    match task_result {
      Ok(results) => {
        for result in results {
          if let Some(ref err) = result.error {
            error_count += 1;
            println!(
              "  ❌ User {} | Reserve {}: {}",
              result.user_address, result.reserve_address, err
            );
            continue;
          }

          // Count no-events gaps (supply=None means no events found, not an error)
          if result.supply.is_none() {
            no_events_count += 1;
          }
          if result.borrow.is_none() {
            no_events_count += 1;
          }

          if let Some(ref supply) = result.supply {
            total_positions += 1;
            if supply.events_vs_chain_pct > 0.01 {
              events_vs_chain_mismatches += 1;
            }
            if supply.db_vs_chain_pct > 0.01 {
              db_vs_chain_mismatches += 1;
            }
            if supply.events_vs_db_pct > 0.01 {
              events_vs_db_mismatches += 1;
            }
            let max_pct = supply
              .events_vs_chain_pct
              .max(supply.db_vs_chain_pct)
              .max(supply.events_vs_db_pct);
            if max_pct > 0.01 {
              worst_offenders.push((
                result.user_address.clone(),
                result.reserve_address.clone(),
                "Supply".to_string(),
                max_pct,
                supply.from_events,
                supply.from_db,
                supply.on_chain,
              ));
            }
          }

          if let Some(ref borrow) = result.borrow {
            total_positions += 1;
            if borrow.events_vs_chain_pct > 0.01 {
              events_vs_chain_mismatches += 1;
            }
            if borrow.db_vs_chain_pct > 0.01 {
              db_vs_chain_mismatches += 1;
            }
            if borrow.events_vs_db_pct > 0.01 {
              events_vs_db_mismatches += 1;
            }
            let max_pct = borrow
              .events_vs_chain_pct
              .max(borrow.db_vs_chain_pct)
              .max(borrow.events_vs_db_pct);
            if max_pct > 0.01 {
              worst_offenders.push((
                result.user_address.clone(),
                result.reserve_address.clone(),
                "Borrow".to_string(),
                max_pct,
                borrow.from_events,
                borrow.from_db,
                borrow.on_chain,
              ));
            }
          }
        }
      }
      Err(e) => {
        error_count += 1;
        eprintln!("Task failed: {}", e);
      }
    }
  }

  // Sort worst offenders by deviation percentage (descending)
  worst_offenders.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));

  // Print summary
  println!("\n📊 Summary:");
  println!("  Total users validated: {}", total_users);
  println!("  Total positions validated: {}", total_positions);
  println!("  Positions with no events (skipped): {}", no_events_count);
  println!(
    "  Events vs Chain mismatches (>0.01%): {}",
    events_vs_chain_mismatches
  );
  println!(
    "  DB vs Chain mismatches (>0.01%): {}",
    db_vs_chain_mismatches
  );
  println!(
    "  Events vs DB mismatches (>0.01%): {}",
    events_vs_db_mismatches
  );
  println!("  Errors: {}", error_count);

  if !worst_offenders.is_empty() {
    println!("\n🔴 Worst offenders (by max deviation %):");
    for (i, (user, reserve, side, pct, from_events, from_db, on_chain)) in
      worst_offenders.iter().take(20).enumerate()
    {
      println!(
        "  {}. {} | {} | {} — {:.4}%",
        i + 1, user, reserve, side, pct
      );
      println!(
        "     Events: {}  DB: {}  Chain: {}",
        from_events, from_db, on_chain
      );
    }
  }

  println!("\n=== Event Replay Validation Complete ===");
}

async fn validate_position_from_events(
  user_address: &str,
  position: &crate::models::UserAssetPositionDocument,
  verbose: bool,
) -> EventValidationResult {
  let reserve_address = &position.reserveAddress;
  let a_token_address = &position.aTokenAddress;
  let debt_token_address = &position.variableDebtTokenAddress;

  // Get DB scaled balances
  let db_supply_scaled: u128 = position
    .aTokenBalance
    .to_string()
    .parse::<u128>()
    .unwrap_or(0);
  let db_borrow_scaled: u128 = position
    .variableDebtTokenBalance
    .to_string()
    .parse::<u128>()
    .unwrap_or(0);

  // === Supply side ===
  let supply_comparison = match validate_side_from_events(
    user_address,
    reserve_address,
    a_token_address,
    db_supply_scaled,
    false, // is_debt
    verbose,
  )
  .await
  {
    Ok(comparison) => comparison, // None = no events found (expected gap)
    Err(e) => {
      if verbose {
        println!(
          "  ⚠️  Supply validation error for reserve {}: {}",
          reserve_address, e
        );
      }
      return EventValidationResult {
        user_address: user_address.to_string(),
        reserve_address: reserve_address.to_string(),
        supply: None,
        borrow: None,
        error: Some(format!("Supply error: {}", e)),
      };
    }
  };

  // === Borrow side ===
  let borrow_comparison = match validate_side_from_events(
    user_address,
    reserve_address,
    debt_token_address,
    db_borrow_scaled,
    true, // is_debt
    verbose,
  )
  .await
  {
    Ok(comparison) => comparison, // None = no events found (expected gap)
    Err(e) => {
      if verbose {
        println!(
          "  ⚠️  Borrow validation error for reserve {}: {}",
          reserve_address, e
        );
      }
      // Return with supply result but borrow error
      return EventValidationResult {
        user_address: user_address.to_string(),
        reserve_address: reserve_address.to_string(),
        supply: supply_comparison,
        borrow: None,
        error: Some(format!("Borrow error: {}", e)),
      };
    }
  };

  EventValidationResult {
    user_address: user_address.to_string(),
    reserve_address: reserve_address.to_string(),
    supply: supply_comparison,
    borrow: borrow_comparison,
    error: None,
  }
}

/// Retries an async RPC call once after a short delay on failure.
async fn retry_rpc<F, Fut, T>(f: F) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
  F: Fn() -> Fut,
  Fut: std::future::Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>>,
{
  match f().await {
    Ok(v) => Ok(v),
    Err(_first_err) => {
      tokio::time::sleep(std::time::Duration::from_secs(2)).await;
      f().await
    }
  }
}

/// Returns None if no events found for this user+token (expected gap, not an error).
async fn validate_side_from_events(
  user_address: &str,
  reserve_address: &str,
  token_address: &str,
  db_scaled_balance: u128,
  is_debt: bool,
  verbose: bool,
) -> Result<Option<ThreeWayComparison>, Box<dyn std::error::Error + Send + Sync>> {
  // Fetch sorted events for this token
  let events = find_token_events_sorted(token_address)
    .await
    .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
      Box::new(std::io::Error::other(
        format!("Failed to fetch events: {}", e),
      ))
    })?;

  // Get current index (with retry for RPC timeouts)
  let reserve_addr = reserve_address.to_string();
  let current_index = if is_debt {
    let r = reserve_addr.clone();
    retry_rpc(|| {
      let r = r.clone();
      async move {
        get_variable_borrow_index(&r)
          .await
          .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            Box::new(std::io::Error::other(
              format!("Failed to fetch borrow index: {}", e),
            ))
          })
      }
    })
    .await?
  } else {
    let r = reserve_addr.clone();
    retry_rpc(|| {
      let r = r.clone();
      async move {
        get_atoken_liquidity_index(&r)
          .await
          .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            Box::new(std::io::Error::other(
              format!("Failed to fetch liquidity index: {}", e),
            ))
          })
      }
    })
    .await?
  };

  // Process events to calculate balance from event replay
  let replay_result = process_user_token_events(&events, user_address, token_address, current_index, verbose)
    .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
      Box::new(std::io::Error::other(
        format!("Failed to process events: {}", e),
      ))
    })?;

  let events_scaled = replay_result.scaled_balance;
  let last_event_block = replay_result.last_event_block;

  // No events found for this user+token — report as gap, not mismatch
  if last_event_block == 0 {
    return Ok(None);
  }

  // Get on-chain scaled balance at last event block (with retry)
  let token_addr = token_address.to_string();
  let user_addr = user_address.to_string();
  let on_chain_scaled = retry_rpc(|| {
    let t = token_addr.clone();
    let u = user_addr.clone();
    async move {
      get_scaled_balance_of(&t, &u, Some(last_event_block))
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
          Box::new(std::io::Error::other(
            format!("Failed to fetch on-chain scaled balance: {}", e),
          ))
        })
    }
  })
  .await?;

  Ok(Some(ThreeWayComparison::new(events_scaled, db_scaled_balance, on_chain_scaled)))
}

fn print_event_validation_result(result: &EventValidationResult) {
  if let Some(ref err) = result.error {
    println!(
      "❌ User {} | Reserve {}: {}",
      result.user_address, result.reserve_address, err
    );
  }

  match &result.supply {
    Some(supply) => print_three_way("Supply", &result.user_address, &result.reserve_address, supply),
    None if result.error.is_none() => println!(
      "  ⏭️  User {} | Reserve {} | Supply: no events found (skipped)",
      result.user_address, result.reserve_address
    ),
    _ => {}
  }

  match &result.borrow {
    Some(borrow) => print_three_way("Borrow", &result.user_address, &result.reserve_address, borrow),
    None if result.error.is_none() => println!(
      "  ⏭️  User {} | Reserve {} | Borrow: no events found (skipped)",
      result.user_address, result.reserve_address
    ),
    _ => {}
  }
}

fn print_three_way(
  side: &str,
  user_address: &str,
  reserve_address: &str,
  comparison: &ThreeWayComparison,
) {
  let status = if comparison.events_vs_chain_diff == 0
    && comparison.db_vs_chain_diff == 0
    && comparison.events_vs_db_diff == 0
  {
    "✅"
  } else if comparison.events_vs_chain_pct < 1.0
    && comparison.db_vs_chain_pct < 1.0
    && comparison.events_vs_db_pct < 1.0
  {
    "⚠️"
  } else {
    "❌"
  };

  println!(
    "\n{} 📊 User {} | Reserve {} | {}",
    status, user_address, reserve_address, side
  );
  println!("  From Events:     {}", comparison.from_events);
  println!("  From DB:         {}", comparison.from_db);
  println!("  On-Chain:        {}", comparison.on_chain);
  println!(
    "  Events vs Chain: {} ({:.4}%)",
    comparison.events_vs_chain_diff, comparison.events_vs_chain_pct
  );
  println!(
    "  DB vs Chain:     {} ({:.4}%)",
    comparison.db_vs_chain_diff, comparison.db_vs_chain_pct
  );
  println!(
    "  Events vs DB:    {} ({:.4}%)",
    comparison.events_vs_db_diff, comparison.events_vs_db_pct
  );
}

// ============================================================================
// --validate-partner-asset
// ============================================================================
//
// Recomputes the canonical `partner_asset` aggregate from `solver_volume`
// (which is correct, unique-indexed) and reports per-row drift against the
// stored values. Used to verify the data-transformator `$inc`-bug from
// sodax-backend issue #498.

const DEFAULT_PARTNER_ASSET_THRESHOLD: f64 = 0.0001;

#[derive(Debug, Clone)]
struct OutputTotals {
  total_fee_in: alloy::primitives::U256,
  total_volume_out: alloy::primitives::U256,
  tx_count: u64,
}

impl OutputTotals {
  fn new() -> Self {
    Self {
      total_fee_in: alloy::primitives::U256::ZERO,
      total_volume_out: alloy::primitives::U256::ZERO,
      tx_count: 0,
    }
  }
}

#[derive(Debug, Clone)]
struct DriftRow {
  receiver: String,
  asset: String,
  chain_id: u64,
  output_token: String,
  stored_cnt: u64,
  computed_cnt: u64,
  stored_fee: String,
  computed_fee: String,
  stored_vol: String,
  computed_vol: String,
  cnt_ratio: Option<f64>,
  fee_ratio: Option<f64>,
  vol_ratio: Option<f64>,
  status: &'static str,
}

fn decimal128_to_u256(d: &mongodb::bson::Decimal128) -> Option<alloy::primitives::U256> {
  alloy::primitives::U256::from_str_radix(&d.to_string(), 10).ok()
}

fn u256_to_f64(value: &alloy::primitives::U256) -> f64 {
  value.to_string().parse::<f64>().unwrap_or(0.0)
}

fn safe_ratio(stored: f64, computed: f64) -> Option<f64> {
  if computed == 0.0 {
    None
  } else {
    Some(stored / computed)
  }
}

fn ratio_within(ratio: Option<f64>, threshold: f64) -> bool {
  match ratio {
    Some(r) => (r - 1.0).abs() <= threshold,
    None => false,
  }
}

pub async fn handle_validate_partner_asset(flags: Vec<Flag>) {
  let partner_filter = extract_optional_value_from_flags(&flags, FlagType::Partner)
    .map(|p| p.to_lowercase());
  let output_json = flags.iter().any(|f| matches!(f, Flag::Json));
  let threshold = flags
    .iter()
    .find_map(|f| match f {
      Flag::Threshold(t) => Some(*t),
      _ => None,
    })
    .unwrap_or(DEFAULT_PARTNER_ASSET_THRESHOLD);

  if !output_json {
    output!("\n=== Partner Asset Validation ===");
    if let Some(ref p) = partner_filter {
      output!("Partner filter: {}", p);
    }
    output!("Threshold: {} (rows within ±{}% are suppressed from table)", threshold, threshold * 100.0);
  }

  // 1. Load solver_volume (authoritative source) and sort by (blockNumber, logIndex)
  //    so that any future $max-style accumulation matches the stored convention.
  let mut solver_rows = match get_solver_volume().await {
    Ok(rows) => rows,
    Err(e) => {
      eprintln!("Error fetching solver_volume: {}", e);
      std::process::exit(1);
    }
  };
  solver_rows.sort_by_key(|r: &SolverVolumeDocument| (r.blockNumber, r.logIndex));

  if !output_json {
    output!("Loaded {} solver_volume rows.", solver_rows.len());
  }

  // 2. Decode + aggregate.
  type Key = (String, String, u64);
  let mut computed: std::collections::HashMap<
    Key,
    std::collections::HashMap<String, OutputTotals>,
  > = std::collections::HashMap::new();
  let mut last_block: std::collections::HashMap<Key, u64> = std::collections::HashMap::new();
  let mut rows_skipped_no_fee: u64 = 0;
  let mut rows_skipped_filter: u64 = 0;
  let mut rows_aggregated: u64 = 0;

  for row in &solver_rows {
    let Some(FeeIntentData { fee, receiver }) = extract_fee_from_intent_data(&row.data) else {
      rows_skipped_no_fee += 1;
      continue;
    };
    let receiver_lower = format!("{:#x}", receiver);
    if let Some(ref pf) = partner_filter
      && &receiver_lower != pf
    {
      rows_skipped_filter += 1;
      continue;
    }
    let asset_lower = row.inputToken.to_lowercase();
    let output_lower = row.outputToken.to_lowercase();
    let key: Key = (receiver_lower, asset_lower, row.chainId);

    let amount = match decimal128_to_u256(&row.amount) {
      Some(a) => a,
      None => {
        eprintln!(
          "Warning: could not parse amount {} as U256 (tx {}); skipping row.",
          row.amount, row.txHash
        );
        continue;
      }
    };

    let bucket = computed
      .entry(key.clone())
      .or_default()
      .entry(output_lower)
      .or_insert_with(OutputTotals::new);
    bucket.total_fee_in += fee;
    bucket.total_volume_out += amount;
    bucket.tx_count += 1;

    let block_entry = last_block.entry(key).or_insert(0);
    if row.blockNumber > *block_entry {
      *block_entry = row.blockNumber;
    }
    rows_aggregated += 1;
  }

  if !output_json {
    output!(
      "Aggregated {} rows; skipped {} without fee data; {} filtered out by --partner.",
      rows_aggregated, rows_skipped_no_fee, rows_skipped_filter
    );
  }

  // 3. Load partner_asset stored docs.
  let stored_docs: Vec<PartnerAssetDocument> = match &partner_filter {
    Some(p) => match find_partner_asset_for_receiver(p).await {
      Ok(d) => d,
      Err(e) => {
        eprintln!("Error fetching partner_asset for receiver {}: {}", p, e);
        std::process::exit(1);
      }
    },
    None => match get_partner_asset().await {
      Ok(d) => d,
      Err(e) => {
        eprintln!("Error fetching partner_asset: {}", e);
        std::process::exit(1);
      }
    },
  };

  // Index stored docs by (receiver_lower, asset_lower, chainId) for O(1) lookup.
  let mut stored: std::collections::HashMap<Key, &PartnerAssetDocument> =
    std::collections::HashMap::new();
  for d in &stored_docs {
    let key: Key = (d.receiver.to_lowercase(), d.asset.to_lowercase(), d.chainId);
    stored.insert(key, d);
  }

  // 4. Build the set of all keys to report on (union of computed + stored).
  let mut all_keys: std::collections::HashSet<Key> = std::collections::HashSet::new();
  for k in computed.keys() {
    all_keys.insert(k.clone());
  }
  for k in stored.keys() {
    all_keys.insert(k.clone());
  }

  // 5. Per (receiver, asset, chainId) × outputToken row build comparison drift entries.
  let mut drift_rows: Vec<DriftRow> = Vec::new();
  for key in &all_keys {
    let empty_outputs: std::collections::HashMap<String, PartnerOutput> =
      std::collections::HashMap::new();
    let stored_outputs = stored
      .get(key)
      .map(|d| &d.outputs)
      .unwrap_or(&empty_outputs);
    let empty_computed: std::collections::HashMap<String, OutputTotals> =
      std::collections::HashMap::new();
    let computed_outputs = computed.get(key).unwrap_or(&empty_computed);

    let mut output_tokens: std::collections::HashSet<String> = std::collections::HashSet::new();
    for ot in stored_outputs.keys() {
      output_tokens.insert(ot.to_lowercase());
    }
    for ot in computed_outputs.keys() {
      output_tokens.insert(ot.clone());
    }

    for output_token in &output_tokens {
      // Stored outputs may be keyed by mixed-case token addresses; do a case-insensitive lookup.
      let stored_entry = stored_outputs
        .iter()
        .find(|(k, _)| k.to_lowercase() == *output_token)
        .map(|(_, v)| v);
      let computed_entry = computed_outputs.get(output_token);

      let stored_cnt = stored_entry.map(|s| s.txCount as u64).unwrap_or(0);
      let computed_cnt = computed_entry.map(|c| c.tx_count).unwrap_or(0);

      let stored_fee_str = stored_entry
        .map(|s| s.totalFeeIn.to_string())
        .unwrap_or_else(|| "0".to_string());
      let computed_fee_str = computed_entry
        .map(|c| c.total_fee_in.to_string())
        .unwrap_or_else(|| "0".to_string());
      let stored_vol_str = stored_entry
        .map(|s| s.totalVolumeOut.to_string())
        .unwrap_or_else(|| "0".to_string());
      let computed_vol_str = computed_entry
        .map(|c| c.total_volume_out.to_string())
        .unwrap_or_else(|| "0".to_string());

      let stored_fee_f = stored_fee_str.parse::<f64>().unwrap_or(0.0);
      let computed_fee_f = computed_entry
        .map(|c| u256_to_f64(&c.total_fee_in))
        .unwrap_or(0.0);
      let stored_vol_f = stored_vol_str.parse::<f64>().unwrap_or(0.0);
      let computed_vol_f = computed_entry
        .map(|c| u256_to_f64(&c.total_volume_out))
        .unwrap_or(0.0);

      let cnt_ratio = safe_ratio(stored_cnt as f64, computed_cnt as f64);
      let fee_ratio = safe_ratio(stored_fee_f, computed_fee_f);
      let vol_ratio = safe_ratio(stored_vol_f, computed_vol_f);

      let status = if computed_cnt > 0 && stored_cnt == 0 {
        "MISSING_IN_STORED"
      } else if stored_cnt > 0 && computed_cnt == 0 {
        "EXTRA_IN_STORED"
      } else if ratio_within(cnt_ratio, threshold)
        && ratio_within(fee_ratio, threshold)
        && ratio_within(vol_ratio, threshold)
      {
        "OK"
      } else {
        "DRIFT"
      };

      drift_rows.push(DriftRow {
        receiver: key.0.clone(),
        asset: key.1.clone(),
        chain_id: key.2,
        output_token: output_token.clone(),
        stored_cnt,
        computed_cnt,
        stored_fee: stored_fee_str,
        computed_fee: computed_fee_str,
        stored_vol: stored_vol_str,
        computed_vol: computed_vol_str,
        cnt_ratio,
        fee_ratio,
        vol_ratio,
        status,
      });
    }
  }

  // Stable ordering for output.
  drift_rows.sort_by(|a, b| {
    a.receiver
      .cmp(&b.receiver)
      .then(a.asset.cmp(&b.asset))
      .then(a.chain_id.cmp(&b.chain_id))
      .then(a.output_token.cmp(&b.output_token))
  });

  // 6. Summary metrics.
  let total_rows = drift_rows.len();
  let rows_ok = drift_rows.iter().filter(|r| r.status == "OK").count();
  let rows_drift = drift_rows.iter().filter(|r| r.status == "DRIFT").count();
  let rows_missing = drift_rows
    .iter()
    .filter(|r| r.status == "MISSING_IN_STORED")
    .count();
  let rows_extra = drift_rows
    .iter()
    .filter(|r| r.status == "EXTRA_IN_STORED")
    .count();
  let rows_over = drift_rows
    .iter()
    .filter(|r| {
      r.cnt_ratio.map(|x| x > 1.0 + threshold).unwrap_or(false)
        || r.fee_ratio.map(|x| x > 1.0 + threshold).unwrap_or(false)
        || r.vol_ratio.map(|x| x > 1.0 + threshold).unwrap_or(false)
    })
    .count();
  let rows_under = drift_rows
    .iter()
    .filter(|r| {
      r.cnt_ratio.map(|x| x < 1.0 - threshold).unwrap_or(false)
        || r.fee_ratio.map(|x| x < 1.0 - threshold).unwrap_or(false)
        || r.vol_ratio.map(|x| x < 1.0 - threshold).unwrap_or(false)
    })
    .count();

  let cnt_ratios: Vec<f64> = drift_rows.iter().filter_map(|r| r.cnt_ratio).collect();
  let max_cnt_ratio = cnt_ratios
    .iter()
    .cloned()
    .fold(f64::NEG_INFINITY, f64::max);
  let min_cnt_ratio = cnt_ratios.iter().cloned().fold(f64::INFINITY, f64::min);
  let unique_cnt_ratios = {
    let mut buckets: Vec<f64> = Vec::new();
    for r in &cnt_ratios {
      let rounded = (r * 1000.0).round() / 1000.0;
      if !buckets.iter().any(|b| (b - rounded).abs() < 1e-9) {
        buckets.push(rounded);
      }
    }
    buckets.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    buckets
  };

  if output_json {
    let rows_json: Vec<_> = drift_rows
      .iter()
      .map(|r| {
        serde_json::json!({
          "receiver": r.receiver,
          "asset": r.asset,
          "chainId": r.chain_id,
          "outputToken": r.output_token,
          "storedCnt": r.stored_cnt,
          "computedCnt": r.computed_cnt,
          "storedFee": r.stored_fee,
          "computedFee": r.computed_fee,
          "storedVol": r.stored_vol,
          "computedVol": r.computed_vol,
          "cntRatio": r.cnt_ratio,
          "feeRatio": r.fee_ratio,
          "volRatio": r.vol_ratio,
          "status": r.status,
        })
      })
      .collect();

    let summary = serde_json::json!({
      "partnerAssetDocs": stored_docs.len(),
      "computedKeys": computed.len(),
      "rowsTotal": total_rows,
      "rowsAgreeingWithinThreshold": rows_ok,
      "rowsWithDrift": rows_drift,
      "rowsMissingInStored": rows_missing,
      "rowsExtraInStored": rows_extra,
      "rowsOverCountedByStored": rows_over,
      "rowsUnderCountedByStored": rows_under,
      "maxCntRatio": if cnt_ratios.is_empty() { None } else { Some(max_cnt_ratio) },
      "minCntRatio": if cnt_ratios.is_empty() { None } else { Some(min_cnt_ratio) },
      "uniqueCntRatios": unique_cnt_ratios,
      "solverVolumeRowsAggregated": rows_aggregated,
      "solverVolumeRowsSkippedNoFee": rows_skipped_no_fee,
      "solverVolumeRowsSkippedFilter": rows_skipped_filter,
      "threshold": threshold,
    });
    let out = serde_json::json!({
      "summary": summary,
      "rows": rows_json,
    });
    output!("{}", serde_json::to_string_pretty(&out).unwrap());
    return;
  }

  // 7. Human-readable table.
  output!(
    "\n{:<44} {:<44} {:<11} {:<44} {:<10} {:<12} {:<9} {:<9} {:<9} {}",
    "RECEIVER", "ASSET", "CHAIN", "OUTPUT_TOKEN", "STORED_CNT", "COMPUTED_CNT",
    "CNT_RATIO", "FEE_RATIO", "VOL_RATIO", "STATUS"
  );
  let mut printed_rows = 0;
  for r in &drift_rows {
    if r.status == "OK" {
      continue;
    }
    let cnt_ratio_s = r
      .cnt_ratio
      .map(|x| format!("{:.4}", x))
      .unwrap_or_else(|| "inf".to_string());
    let fee_ratio_s = r
      .fee_ratio
      .map(|x| format!("{:.4}", x))
      .unwrap_or_else(|| "inf".to_string());
    let vol_ratio_s = r
      .vol_ratio
      .map(|x| format!("{:.4}", x))
      .unwrap_or_else(|| "inf".to_string());
    output!(
      "{:<44} {:<44} {:<11} {:<44} {:<10} {:<12} {:<9} {:<9} {:<9} {}",
      r.receiver, r.asset, r.chain_id, r.output_token,
      r.stored_cnt, r.computed_cnt,
      cnt_ratio_s, fee_ratio_s, vol_ratio_s, r.status
    );
    printed_rows += 1;
  }
  if printed_rows == 0 {
    output!("(no rows outside threshold)");
  }

  // Summary block.
  output!("\nSUMMARY:");
  output!("  partner_asset docs:                {}", stored_docs.len());
  output!("  computed (receiver, asset) keys:   {}", computed.len());
  output!("  rows total:                        {}", total_rows);
  output!("  rows agreeing within threshold:    {} / {}", rows_ok, total_rows);
  output!("  rows with drift:                   {}", rows_drift);
  output!("  rows missing in partner_asset:     {}", rows_missing);
  output!("  rows extra in partner_asset:       {} (no solver_volume backing)", rows_extra);
  output!("  rows over-counted by stored:       {}", rows_over);
  output!("  rows under-counted by stored:      {} (should be 0; investigate if not)", rows_under);
  output!("  solver_volume rows aggregated:     {}", rows_aggregated);
  output!("  solver_volume rows skipped (no fee data): {}", rows_skipped_no_fee);
  if cnt_ratios.is_empty() {
    output!("  max CNT_RATIO observed:            n/a");
    output!("  min CNT_RATIO observed:            n/a");
  } else {
    output!("  max CNT_RATIO observed:            {:.4}", max_cnt_ratio);
    output!("  min CNT_RATIO observed:            {:.4}", min_cnt_ratio);
  }
  let unique_s: Vec<String> = unique_cnt_ratios.iter().map(|r| format!("{:.3}", r)).collect();
  output!("  unique CNT_RATIO values:           [{}]", unique_s.join(", "));
  output!("\n=== Partner Asset Validation Complete ===");
}

// ============================================================
// --calculate-from-events-reserve handler
// ============================================================

#[derive(Debug, Clone)]
struct ReserveReplayRow {
  user: String,
  /// Scaled balance from event replay (the "DB events view").
  db_scaled: u128,
  /// Scaled balance read from the `user_positions` collection for this (user, reserve, side).
  /// `Ok` on hit; `Err` if the user_positions doc is missing, has no entry for this reserve,
  /// or the prefetch itself failed. Surfaced as a separate column in the report.
  position_scaled: Result<u128, String>,
  /// Scaled balance from on-chain `scaledBalanceOf`, pinned to `last_event_block`.
  chain_scaled: u128,
  diff: u128,
  pct: f64,
  last_event_block: u64,
  error: Option<String>,
}

impl ReserveReplayRow {
  fn verdict(&self) -> &'static str {
    classify_verdict(self.diff, self.chain_scaled, self.error.is_some())
  }

  fn verdict_symbol(&self) -> &'static str {
    match self.verdict() {
      "PERFECT" | "EXCELLENT" => "✅",
      "MINOR" => "⚠️",
      "ERROR" => "‼️",
      _ => "❌",
    }
  }
}

/// Classifies a (diff, baseline) pair into a verdict using integer math.
///
/// The `pct` field on `ReserveReplayRow` is f64 for display, but f64 loses precision once
/// values exceed ~2^53 — large enough that a balance comparison could flip buckets purely
/// from rounding. This function compares against the 1% and 0.01% thresholds with u128
/// arithmetic so the verdict stays exact regardless of balance magnitude.
///
/// `baseline` is the denominator — in this command's caller it's the on-chain scaled
/// balance, so the percentage is "how far does the replayed value drift from chain?"
///
/// Semantics:
/// - error set → `"ERROR"` (precedence over numeric verdict)
/// - diff == 0 → `"PERFECT"`
/// - baseline == 0 with diff > 0 → `"SIGNIFICANT"` (db replay produced a balance, chain has none).
///   Note: this differs from `--calculate-from-events`, which reports such cases as 0% /
///   "Excellent". The reserve handler aligns with `EntryState::new` in `structs.rs` instead.
/// - diff/baseline ≥ 1/100 → `"SIGNIFICANT"`   (≥ 1%)
/// - diff/baseline ≥ 1/10000 → `"MINOR"`       (≥ 0.01%)
/// - otherwise → `"EXCELLENT"`
///
/// `checked_mul` overflows are treated as threshold-met (a `diff` so large that
/// `diff * 100` or `diff * 10000` overflows u128 implies the ratio against `baseline` is
/// astronomically high — well past any threshold).
fn classify_verdict(diff: u128, baseline: u128, has_error: bool) -> &'static str {
  if has_error {
    return "ERROR";
  }
  if diff == 0 {
    return "PERFECT";
  }
  if baseline == 0 {
    return "SIGNIFICANT";
  }
  // diff/baseline ≥ 1/100  ⟺  diff*100 ≥ baseline
  let geq_1pct = diff
    .checked_mul(100)
    .is_none_or(|v| v >= baseline);
  if geq_1pct {
    return "SIGNIFICANT";
  }
  // diff/baseline ≥ 1/10000  ⟺  diff*10000 ≥ baseline
  let geq_0_01pct = diff
    .checked_mul(10000)
    .is_none_or(|v| v >= baseline);
  if geq_0_01pct {
    return "MINOR";
  }
  "EXCELLENT"
}

/// Per-user user_positions cache value. We need read-only access to the user's positions
/// array (one entry per reserve they hold), keyed by reserve address.
type CachedPositions = Result<Arc<Vec<UserAssetPositionDocument>>, String>;

/// Per-user user_positions cache. Keyed by lowercased address. Mirrors EventsCache.
type PositionsCache = std::collections::HashMap<String, CachedPositions>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PositionSide {
  Supply,
  Borrow,
}

const RESERVE_REPLAY_CONCURRENCY: usize = 10;

fn err_row(user: String, msg: String) -> ReserveReplayRow {
  ReserveReplayRow {
    user,
    db_scaled: 0,
    position_scaled: Err("not fetched (row errored before position lookup)".to_string()),
    chain_scaled: 0,
    diff: 0,
    pct: 0.0,
    last_event_block: 0,
    error: Some(msg),
  }
}

/// Fetch each user's `user_positions` document once with bounded fan-out. Stores the
/// returned positions array per user (across all reserves they hold) — we filter to the
/// matching reserve at lookup time.
async fn prefetch_user_positions(users: &[String]) -> PositionsCache {
  use futures::stream::{self, StreamExt};

  let results: Vec<(String, CachedPositions)> = stream::iter(users.iter().cloned())
    .map(|user| async move {
      let res = find_user_assets_position(&user)
        .await
        .map(Arc::new)
        .map_err(|e| e.to_string());
      (user.to_lowercase(), res)
    })
    .buffered(RESERVE_REPLAY_CONCURRENCY)
    .collect()
    .await;

  let mut map: PositionsCache = std::collections::HashMap::with_capacity(results.len());
  for (key, res) in results {
    map.insert(key, res);
  }
  map
}

/// Look up the scaled balance recorded in `user_positions` for this (user, reserve, side).
/// Returns:
/// - `Ok(scaled)` on hit.
/// - `Err(_)` if the prefetch failed for this user, the user has no positions document,
///   or the document has no entry for this reserve.
fn lookup_position_scaled(
  user: &str,
  reserve_address: &str,
  side: PositionSide,
  cache: &PositionsCache,
) -> Result<u128, String> {
  let positions = match cache.get(&user.to_lowercase()) {
    Some(Ok(p)) => p,
    Some(Err(e)) => return Err(format!("fetch position: {}", e)),
    None => return Err("position not in prefetch cache".to_string()),
  };

  let reserve_lc = reserve_address.to_lowercase();
  let pos = positions
    .iter()
    .find(|p| p.reserveAddress.to_lowercase() == reserve_lc);
  let pos = match pos {
    Some(p) => p,
    None => return Err(format!("no position for reserve {}", reserve_address)),
  };

  let balance = match side {
    PositionSide::Supply => pos.aTokenBalance,
    PositionSide::Borrow => pos.variableDebtTokenBalance,
  };
  Ok(decimal128_to_u128(balance))
}

async fn replay_one_user(
  user_address: String,
  token_address: String,
  verbose: bool,
  token_events: Arc<Vec<MoneyMarketEventDocument>>,
  position_scaled: Result<u128, String>,
  current_index_for_display: u128,
) -> ReserveReplayRow {
  // We pass the full token-event stream (all users, sorted by block/logIndex), not just
  // events involving our user. `process_user_token_events` filters to the target user
  // inside, but it crucially also updates `last_index` from any mint/burn event on the
  // token before that filter — so when our user's first event for the token is a
  // transfer, `last_index` already reflects the pool's liquidity index at that block
  // (set by some earlier user's mint/burn), instead of staying stuck at RAY=1.0 and
  // crediting an inflated scaled balance.
  //
  // The `current_index_for_display` argument is purely informational — it's only consulted
  // by the function's verbose printout. The scaled-vs-scaled comparison does not use it.
  // In verbose mode the caller fetches the real current index from chain and passes it
  // here; in non-verbose mode the caller passes RAY since no one reads it.
  let result = match process_user_token_events(&token_events, &user_address, &token_address, current_index_for_display, verbose) {
    Ok(r) => r,
    Err(e) => {
      let mut row = err_row(user_address, format!("replay: {}", e));
      row.position_scaled = position_scaled;
      return row;
    }
  };

  // If the replay found no matching events for this user × token, last_event_block is 0.
  // That means the user is in suppliers/borrowers but the token-event stream has no entry
  // touching them — itself a data integrity signal worth surfacing. Treat as ERROR rather
  // than silently degrading to a "compare at latest block" call (which would be unpinned
  // and could read a balance reflecting accrued interest the replay never saw).
  if result.last_event_block == 0 {
    return ReserveReplayRow {
      user: user_address,
      db_scaled: result.scaled_balance,
      position_scaled,
      chain_scaled: 0,
      diff: 0,
      pct: 0.0,
      last_event_block: 0,
      error: Some(
        "no events found for user in token-event stream (cannot pin chain comparison to a block)".to_string(),
      ),
    };
  }

  // Compare scaled balance from replay vs. scaled balance on-chain, both pinned to the
  // user's last event block. This sidesteps the index entirely — no f64 conversion, no
  // index-timing question. Discrepancies are pure data integrity issues.
  let block = Some(result.last_event_block);
  let chain_scaled = match get_scaled_balance_of(&token_address, &user_address, block).await {
    Ok(b) => b,
    Err(e) => {
      return ReserveReplayRow {
        user: user_address,
        db_scaled: result.scaled_balance,
        position_scaled,
        chain_scaled: 0,
        diff: 0,
        pct: 0.0,
        last_event_block: result.last_event_block,
        error: Some(format!("on-chain scaledBalanceOf: {}", e)),
      };
    }
  };

  let diff = result.scaled_balance.abs_diff(chain_scaled);
  let pct = if chain_scaled == 0 {
    if diff == 0 { 0.0 } else { 100.0 }
  } else {
    (diff as f64 / chain_scaled as f64) * 100.0
  };

  ReserveReplayRow {
    user: user_address,
    db_scaled: result.scaled_balance,
    position_scaled,
    chain_scaled,
    diff,
    pct,
    last_event_block: result.last_event_block,
    error: None,
  }
}

#[allow(clippy::too_many_arguments)]
async fn replay_side(
  users: &[String],
  token_address: &str,
  reserve_address: &str,
  side: PositionSide,
  verbose: bool,
  token_events: Arc<Vec<MoneyMarketEventDocument>>,
  positions_cache: &PositionsCache,
  current_index_for_display: u128,
) -> Vec<ReserveReplayRow> {
  use futures::stream::{self, StreamExt};

  // Verbose mode: per-event output! lines from concurrent users would interleave, making
  // the "per-user replay block" unreadable. Run sequentially so each block stays contiguous.
  if verbose {
    let mut rows = Vec::with_capacity(users.len());
    for user in users {
      let position = lookup_position_scaled(user, reserve_address, side, positions_cache);
      let row = replay_one_user(
        user.clone(),
        token_address.to_string(),
        true,
        Arc::clone(&token_events),
        position,
        current_index_for_display,
      )
      .await;
      rows.push(row);
    }
    return rows;
  }

  // Concurrent path. `buffered` (vs. `buffer_unordered`) preserves input order so the
  // emitted table / JSON is deterministic across runs on the same data — important for
  // downstream tooling (snapshot tests, diff-based monitoring). At most
  // RESERVE_REPLAY_CONCURRENCY futures in flight; no upfront fan-out.
  stream::iter(users.iter().cloned())
    .map(|user| {
      let position = lookup_position_scaled(&user, reserve_address, side, positions_cache);
      let token_address = token_address.to_string();
      let events = Arc::clone(&token_events);
      async move { replay_one_user(user, token_address, false, events, position, current_index_for_display).await }
    })
    .buffered(RESERVE_REPLAY_CONCURRENCY)
    .collect()
    .await
}

fn print_replay_table(label: &str, token_address: &str, rows: &[ReserveReplayRow]) {
  output!("\n=== {} ({}) ===", label, token_address);
  if rows.is_empty() {
    output!("(no users)");
    return;
  }
  output!(
    "{:<44} {:>26} {:>26} {:>26} {:>10} {}",
    "User", "DB Events Scaled", "Position Scaled", "Chain Scaled", "Diff%", "Verdict"
  );
  for row in rows {
    let position_cell = match &row.position_scaled {
      Ok(v) => v.to_string(),
      Err(_) => "-".to_string(),
    };
    if let Some(err) = &row.error {
      output!(
        "{:<44} {:>26} {:>26} {:>26} {:>10} {} {}",
        row.user, "-", position_cell, "-", "-", row.verdict_symbol(), err
      );
    } else {
      output!(
        "{:<44} {:>26} {:>26} {:>26} {:>9.4}% {}",
        row.user, row.db_scaled, position_cell, row.chain_scaled, row.pct, row.verdict_symbol()
      );
    }
  }

  let mut perfect = 0usize;
  let mut excellent = 0usize;
  let mut minor = 0usize;
  let mut significant = 0usize;
  let mut error_count = 0usize;
  for row in rows {
    match row.verdict() {
      "PERFECT" => perfect += 1,
      "EXCELLENT" => excellent += 1,
      "MINOR" => minor += 1,
      "SIGNIFICANT" => significant += 1,
      "ERROR" => error_count += 1,
      _ => {}
    }
  }
  output!(
    "\nSummary ({}): {} users  ·  ✅ {} perfect / {} excellent  ·  ⚠️ {} minor  ·  ❌ {} significant  ·  ‼️ {} errors",
    label,
    rows.len(),
    perfect,
    excellent,
    minor,
    significant,
    error_count,
  );
}

fn rows_to_json(rows: &[ReserveReplayRow]) -> Vec<serde_json::Value> {
  rows
    .iter()
    .map(|r| {
      let (position_scaled, position_error) = match &r.position_scaled {
        Ok(v) => (serde_json::Value::String(v.to_string()), serde_json::Value::Null),
        Err(e) => (serde_json::Value::Null, serde_json::Value::String(e.clone())),
      };
      serde_json::json!({
        "user": r.user,
        "dbScaled": r.db_scaled.to_string(),
        "positionScaled": position_scaled,
        "positionError": position_error,
        "chainScaled": r.chain_scaled.to_string(),
        "diff": r.diff.to_string(),
        "percentage": r.pct,
        "lastEventBlock": r.last_event_block,
        "verdict": r.verdict(),
        "error": r.error,
      })
    })
    .collect()
}

fn rows_summary_json(rows: &[ReserveReplayRow]) -> serde_json::Value {
  let mut perfect = 0u64;
  let mut excellent = 0u64;
  let mut minor = 0u64;
  let mut significant = 0u64;
  let mut error_count = 0u64;
  for r in rows {
    match r.verdict() {
      "PERFECT" => perfect += 1,
      "EXCELLENT" => excellent += 1,
      "MINOR" => minor += 1,
      "SIGNIFICANT" => significant += 1,
      "ERROR" => error_count += 1,
      _ => {}
    }
  }
  serde_json::json!({
    "totalUsers": rows.len(),
    "perfect": perfect,
    "excellent": excellent,
    "minor": minor,
    "significant": significant,
    "errors": error_count,
  })
}

pub async fn handle_calculate_from_events_reserve(flags: Vec<Flag>) {
  let reserve_address_raw = extract_value_from_flags_or_exit(
    flags.clone(),
    FlagType::CalculateFromEventsReserve,
    "Error: --calculate-from-events-reserve requires a reserve address to be specified.",
  );

  let a_token_only = flags.iter().any(|f| matches!(f, Flag::ATokenOnly));
  let debt_token_only = flags.iter().any(|f| matches!(f, Flag::DebtTokenOnly));
  let verbose = flags.iter().any(|f| matches!(f, Flag::Verbose));
  let output_json = flags.iter().any(|f| matches!(f, Flag::Json));

  let do_supply = !debt_token_only;
  let do_borrow = !a_token_only;

  // Resolve reserve data
  // reserve_tokens.reserveAddress is stored lowercase (see docs/sodax-backend/COLLECTIONS.md),
  // and find_reserve_for_token does an exact match — so checksummed / mixed-case input
  // would silently fail to find a real reserve. Normalize once here.
  let reserve_address_lc = reserve_address_raw.to_lowercase();
  let reserve_data = match find_reserve_for_token(&reserve_address_lc, ReserveTokenField::Reserve).await {
    Ok(Some(data)) => data,
    Ok(None) => {
      eprintln!("Reserve not found: {}", reserve_address_raw);
      std::process::exit(1);
    }
    Err(e) => {
      eprintln!("Error fetching reserve data: {}", e);
      std::process::exit(1);
    }
  };

  // No reserve index fetch needed for the verdict — we compare scaled balances directly
  // (replay's scaled_balance vs. on-chain scaledBalanceOf), so the liquidity /
  // variableBorrowIndex never enters the comparison. The verdict math sidesteps every
  // index-timing question and the f64 precision loss path for large balances.
  //
  // We still fetch the current pool indexes when --verbose is set so the per-user replay
  // header (printed by process_user_token_events) shows a real number instead of the RAY
  // placeholder. The number is informational and not used in the verdict.
  let want_index_for_verbose = verbose && !output_json;
  let supply_current_index = if do_supply && want_index_for_verbose {
    get_atoken_liquidity_index(&reserve_data.reserveAddress)
      .await
      .unwrap_or(RAY)
  } else {
    RAY
  };
  let borrow_current_index = if do_borrow && want_index_for_verbose {
    get_variable_borrow_index(&reserve_data.reserveAddress)
      .await
      .unwrap_or(RAY)
  } else {
    RAY
  };

  let mode_label = if a_token_only {
    "supply only"
  } else if debt_token_only {
    "borrow only"
  } else {
    "supply + borrow"
  };

  if !output_json {
    output!("\n=== Reserve Event-Replay Reconstruction ===");
    output!("Reserve: {} ({})", reserve_data.reserveAddress, reserve_data.symbol);
    output!("aToken:  {}", reserve_data.aTokenAddress);
    output!("Debt:    {}", reserve_data.variableDebtTokenAddress);
    output!("Mode:    {}", mode_label);
    output!("Suppliers: {} · Borrowers: {}", reserve_data.suppliers.len(), reserve_data.borrowers.len());
  }

  // Prefetch events once for the union of suppliers + borrowers so cross-side users
  // (in both lists) don't trigger duplicate DB fetches when both sides are selected.
  // De-dup is case-insensitive so addresses that differ only in casing across the two
  // lists collapse to a single entry (matching how the cache keys are stored).
  let unique_users: Vec<String> = {
    let mut set: std::collections::HashSet<String> = std::collections::HashSet::new();
    if do_supply {
      for u in &reserve_data.suppliers {
        set.insert(u.to_lowercase());
      }
    }
    if do_borrow {
      for u in &reserve_data.borrowers {
        set.insert(u.to_lowercase());
      }
    }
    set.into_iter().collect()
  };
  if !output_json {
    output!("Prefetching user_positions for {} unique users…", unique_users.len());
  }
  let positions_cache = prefetch_user_positions(&unique_users).await;

  // Fetch the full token-event stream once per side. This is N=1 DB query per side
  // regardless of user count and — critically — captures mints/burns by *every* user
  // for this token. `process_user_token_events` filters to the target user internally,
  // but sees the full stream first, so `last_index` is always up-to-date by the time
  // a user's transfer event lands. (Per-user event fetches missed cross-user mints
  // that defined the pool's liquidity index at transfer time, which previously caused
  // transfers-as-first-event to credit an inflated scaled balance.)
  let supply_token_events = if do_supply {
    if !output_json {
      output!("\nFetching aToken events…");
    }
    match find_token_events_sorted(&reserve_data.aTokenAddress).await {
      Ok(events) => Some(Arc::new(events)),
      Err(e) => {
        eprintln!("Error fetching aToken events: {}", e);
        std::process::exit(1);
      }
    }
  } else {
    None
  };
  let borrow_token_events = if do_borrow {
    if !output_json {
      output!("Fetching variable debt token events…");
    }
    match find_token_events_sorted(&reserve_data.variableDebtTokenAddress).await {
      Ok(events) => Some(Arc::new(events)),
      Err(e) => {
        eprintln!("Error fetching variable debt token events: {}", e);
        std::process::exit(1);
      }
    }
  } else {
    None
  };

  // Run each selected side. In verbose mode, process_user_token_events prints per-event
  // detail as it runs, so we suppress the compact table afterward to avoid duplication.
  let supply_rows = if do_supply {
    if !output_json {
      output!("\nReplaying {} suppliers (aToken)…", reserve_data.suppliers.len());
    }
    replay_side(
      &reserve_data.suppliers,
      &reserve_data.aTokenAddress,
      &reserve_data.reserveAddress,
      PositionSide::Supply,
      verbose && !output_json,
      supply_token_events.clone().expect("supply_token_events set when do_supply"),
      &positions_cache,
      supply_current_index,
    )
    .await
  } else {
    Vec::new()
  };

  let borrow_rows = if do_borrow {
    if !output_json {
      output!("\nReplaying {} borrowers (variable debt)…", reserve_data.borrowers.len());
    }
    replay_side(
      &reserve_data.borrowers,
      &reserve_data.variableDebtTokenAddress,
      &reserve_data.reserveAddress,
      PositionSide::Borrow,
      verbose && !output_json,
      borrow_token_events.clone().expect("borrow_token_events set when do_borrow"),
      &positions_cache,
      borrow_current_index,
    )
    .await
  } else {
    Vec::new()
  };

  if output_json {
    let mut out = serde_json::json!({
      "reserve": reserve_data.reserveAddress,
      "symbol": reserve_data.symbol,
      "aTokenAddress": reserve_data.aTokenAddress,
      "variableDebtTokenAddress": reserve_data.variableDebtTokenAddress,
      "mode": mode_label,
      "comparison": "scaled-vs-scaled",
    });
    if do_supply {
      out["supply"] = serde_json::json!({
        "users": rows_to_json(&supply_rows),
        "summary": rows_summary_json(&supply_rows),
      });
    }
    if do_borrow {
      out["borrow"] = serde_json::json!({
        "users": rows_to_json(&borrow_rows),
        "summary": rows_summary_json(&borrow_rows),
      });
    }
    output!("{}", serde_json::to_string_pretty(&out).unwrap());
    return;
  }

  if verbose {
    // Per-user details were already printed inline by process_user_token_events.
    output!("\n(verbose mode: per-user replay detail printed above)");
  } else {
    if do_supply {
      print_replay_table("Supply", &reserve_data.aTokenAddress, &supply_rows);
    }
    if do_borrow {
      print_replay_table("Borrow", &reserve_data.variableDebtTokenAddress, &borrow_rows);
    }
  }

  output!("\n=== Reserve Event-Replay Reconstruction Complete ===");
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn outgoing_transfer_gets_from_suffix() {
    let id = reconstruct_balance_event_id(
      55211680,
      "0x004493f6d37af994c464920f64f657b9425092db69ac9b96714f9c3e91342e44",
      102,
      Some("0x996752752F887000C8136ceFB023d12719DAD24a"),
      "0x996752752f887000c8136cefb023d12719dad24a",
    );
    assert_eq!(
      id,
      "55211680-0x004493f6d37af994c464920f64f657b9425092db69ac9b96714f9c3e91342e44-102-from"
    );
  }

  #[test]
  fn incoming_transfer_uses_plain_form() {
    let id = reconstruct_balance_event_id(
      55211680,
      "0x004493f6d37af994c464920f64f657b9425092db69ac9b96714f9c3e91342e44",
      102,
      Some("0xf2E26765949731f251D5d15f30f483b7a321b3A4"),
      "0x996752752f887000c8136cefb023d12719dad24a",
    );
    assert_eq!(
      id,
      "55211680-0x004493f6d37af994c464920f64f657b9425092db69ac9b96714f9c3e91342e44-102"
    );
  }

  #[test]
  fn non_transfer_event_uses_plain_form() {
    let id = reconstruct_balance_event_id(
      100,
      "0xabc",
      5,
      None,
      "0x996752752f887000c8136cefb023d12719dad24a",
    );
    assert_eq!(id, "100-0xabc-5");
  }

  // ----------------------------------------------------------
  // --calculate-from-events-reserve verdict / JSON tests
  // ----------------------------------------------------------

  /// Build a row where diff/chain_scaled falls into the target verdict bucket.
  /// `pct` is set consistently for display only — verdict reads the integer fields.
  fn ok_row(diff: u128, chain_scaled: u128) -> ReserveReplayRow {
    let pct = if chain_scaled == 0 {
      if diff == 0 { 0.0 } else { 100.0 }
    } else {
      (diff as f64 / chain_scaled as f64) * 100.0
    };
    ReserveReplayRow {
      user: "0xuser".to_string(),
      db_scaled: 0,
      position_scaled: Ok(0),
      chain_scaled,
      diff,
      pct,
      last_event_block: 100,
      error: None,
    }
  }

  #[test]
  fn verdict_perfect_when_diff_zero() {
    let row = ok_row(0, 100);
    assert_eq!(row.verdict(), "PERFECT");
    assert_eq!(row.verdict_symbol(), "✅");
  }

  #[test]
  fn verdict_excellent_below_thresh_0_01() {
    // 1/20_000 = 0.005% — well under the 0.01% EXCELLENT cutoff.
    let row = ok_row(1, 20_000);
    assert_eq!(row.verdict(), "EXCELLENT");
    assert_eq!(row.verdict_symbol(), "✅");
  }

  #[test]
  fn verdict_minor_below_thresh_1_pct() {
    // 5/10_000 = 0.05% — between the EXCELLENT and SIGNIFICANT thresholds.
    let row = ok_row(5, 10_000);
    assert_eq!(row.verdict(), "MINOR");
    assert_eq!(row.verdict_symbol(), "⚠️");
  }

  #[test]
  fn verdict_significant_at_or_above_1_pct() {
    // 1/100 = exactly 1% → SIGNIFICANT (≥ branch).
    let at = ok_row(1, 100);
    assert_eq!(at.verdict(), "SIGNIFICANT");
    assert_eq!(at.verdict_symbol(), "❌");
    let above = ok_row(100, 200);
    assert_eq!(above.verdict(), "SIGNIFICANT");
  }

  #[test]
  fn verdict_error_when_error_set() {
    let mut row = ok_row(0, 0);
    row.error = Some("boom".to_string());
    assert_eq!(row.verdict(), "ERROR");
    assert_eq!(row.verdict_symbol(), "‼️");
  }

  #[test]
  fn verdict_error_dominates_zero_diff() {
    // Even if numerically perfect, an error should classify as ERROR.
    let mut row = ok_row(0, 100);
    row.error = Some("fetch failed".to_string());
    assert_eq!(row.verdict(), "ERROR");
  }

  #[test]
  fn boundary_just_under_excellent_threshold() {
    // 1/10_001 ≈ 0.0099% — just under the 0.01% threshold.
    let row = ok_row(1, 10_001);
    assert_eq!(row.verdict(), "EXCELLENT");
  }

  #[test]
  fn rows_summary_counts_each_bucket() {
    let rows = vec![
      ok_row(0, 100),                // PERFECT
      ok_row(1, 20_000),             // EXCELLENT
      ok_row(5, 10_000),             // MINOR
      ok_row(1, 100),                // SIGNIFICANT (exactly 1%)
      ok_row(50, 100),               // SIGNIFICANT (50%)
      {
        let mut r = ok_row(0, 0);
        r.error = Some("err".to_string());
        r
      },
    ];
    let summary = rows_summary_json(&rows);
    assert_eq!(summary["totalUsers"], 6);
    assert_eq!(summary["perfect"], 1);
    assert_eq!(summary["excellent"], 1);
    assert_eq!(summary["minor"], 1);
    assert_eq!(summary["significant"], 2);
    assert_eq!(summary["errors"], 1);
  }

  #[test]
  fn rows_to_json_emits_u128_as_strings() {
    // diff=7, chain_scaled=42: ratio ≈ 16.7% → SIGNIFICANT under integer math.
    let row = ReserveReplayRow {
      user: "0xabc".to_string(),
      db_scaled: u128::MAX,
      position_scaled: Ok(u128::MAX - 1),
      chain_scaled: 42,
      diff: 7,
      pct: 16.666_666_666_666_668,
      last_event_block: 12345,
      error: None,
    };
    let arr = rows_to_json(&[row]);
    let v = &arr[0];
    // u128 fields are stringified so they survive JSON without precision loss.
    assert_eq!(v["user"], "0xabc");
    assert_eq!(v["dbScaled"], u128::MAX.to_string());
    assert_eq!(v["positionScaled"], (u128::MAX - 1).to_string());
    assert!(v["positionError"].is_null());
    assert_eq!(v["chainScaled"], "42");
    assert_eq!(v["diff"], "7");
    assert_eq!(v["percentage"], 16.666_666_666_666_668);
    assert_eq!(v["lastEventBlock"], 12345);
    assert_eq!(v["verdict"], "SIGNIFICANT");
    assert!(v["error"].is_null());
  }

  #[test]
  fn rows_to_json_preserves_error_message() {
    let row = ReserveReplayRow {
      user: "0xdef".to_string(),
      db_scaled: 0,
      position_scaled: Err("no position for reserve 0xreserve".to_string()),
      chain_scaled: 0,
      diff: 0,
      pct: 0.0,
      last_event_block: 0,
      error: Some("on-chain scaledBalanceOf: rpc unreachable".to_string()),
    };
    let arr = rows_to_json(&[row]);
    let v = &arr[0];
    assert_eq!(v["verdict"], "ERROR");
    assert_eq!(v["error"], "on-chain scaledBalanceOf: rpc unreachable");
    // Position fields surface independently of the main error.
    assert!(v["positionScaled"].is_null());
    assert_eq!(v["positionError"], "no position for reserve 0xreserve");
  }

  #[test]
  fn rows_to_json_emits_position_error_as_string() {
    // Even when the row is otherwise green, a position lookup failure surfaces in
    // positionError so the report can flag "events match chain but user_positions is
    // missing" cases.
    let row = ReserveReplayRow {
      user: "0xghi".to_string(),
      db_scaled: 100,
      position_scaled: Err("no position for reserve 0xunknown".to_string()),
      chain_scaled: 100,
      diff: 0,
      pct: 0.0,
      last_event_block: 50,
      error: None,
    };
    let arr = rows_to_json(&[row]);
    let v = &arr[0];
    assert_eq!(v["verdict"], "PERFECT");
    assert!(v["positionScaled"].is_null());
    assert_eq!(v["positionError"], "no position for reserve 0xunknown");
  }

  // ----------------------------------------------------------
  // classify_verdict (integer math, no f64) edge cases
  // ----------------------------------------------------------

  #[test]
  fn classify_error_takes_precedence_over_perfect() {
    assert_eq!(classify_verdict(0, 0, true), "ERROR");
    assert_eq!(classify_verdict(0, 100, true), "ERROR");
  }

  #[test]
  fn classify_perfect_when_diff_zero() {
    assert_eq!(classify_verdict(0, 0, false), "PERFECT");
    assert_eq!(classify_verdict(0, u128::MAX, false), "PERFECT");
  }

  #[test]
  fn classify_on_chain_zero_with_diff_is_significant() {
    // db has a balance but chain reports zero: aligns with EntryState::new in structs.rs.
    assert_eq!(classify_verdict(1, 0, false), "SIGNIFICANT");
    assert_eq!(classify_verdict(u128::MAX, 0, false), "SIGNIFICANT");
  }

  #[test]
  fn classify_thresholds_around_1_percent() {
    // 1.0% exactly (diff*100 == on_chain) → SIGNIFICANT
    assert_eq!(classify_verdict(1, 100, false), "SIGNIFICANT");
    // Just under 1.0% (diff*100 < on_chain) → MINOR
    assert_eq!(classify_verdict(99, 10_000, false), "MINOR");
  }

  #[test]
  fn classify_thresholds_around_0_01_percent() {
    // 0.01% exactly (diff*10000 == on_chain) → MINOR
    assert_eq!(classify_verdict(1, 10_000, false), "MINOR");
    // Just under 0.01% → EXCELLENT
    assert_eq!(classify_verdict(99, 1_000_000, false), "EXCELLENT");
  }

  #[test]
  fn classify_large_balances_precise_at_threshold() {
    // Values well past 2^53 (f64's safe integer range) where the float pct would round
    // ambiguously. Integer math still decides correctly.
    //
    // diff = 1e20, on_chain = 1e22 → ratio 1/100 = exactly 1% → SIGNIFICANT
    let diff = 100_000_000_000_000_000_000u128;
    let on_chain = 10_000_000_000_000_000_000_000u128;
    assert_eq!(classify_verdict(diff, on_chain, false), "SIGNIFICANT");

    // Same scale but one unit under threshold — should be MINOR, not SIGNIFICANT.
    let diff_minus = diff - 1;
    assert_eq!(classify_verdict(diff_minus, on_chain, false), "MINOR");
  }

  #[test]
  fn classify_overflow_on_diff_times_100_means_significant() {
    // diff so large that diff*100 overflows u128. Any sane on_chain is dwarfed → SIGNIFICANT.
    let diff = u128::MAX / 50; // diff*100 overflows
    assert_eq!(classify_verdict(diff, 1, false), "SIGNIFICANT");
    assert_eq!(classify_verdict(diff, u128::MAX, false), "SIGNIFICANT");
  }
}
