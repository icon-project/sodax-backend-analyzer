use crate::constants::RAY;
use crate::models::MoneyMarketEventDocument;
use mongodb::bson::Decimal128;
use primitive_types::U256;

const ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";

#[derive(Debug, Clone)]
pub struct BalanceResult {
  pub scaled_balance: u128,
  pub real_balance: u128,
  pub last_index: u128,
  pub last_event_block: u64,
}

/// Event type for calculation purposes
#[derive(Debug, Clone, Copy, PartialEq)]
enum EventType {
  Mint,
  Burn,
  Transfer,
}

/// Gets the token address from an event
fn get_event_token_address(event: &MoneyMarketEventDocument) -> Option<&str> {
  match event {
    MoneyMarketEventDocument::ATokenMint(e) => Some(&e.tokenAddress),
    MoneyMarketEventDocument::ATokenBurn(e) => Some(&e.tokenAddress),
    MoneyMarketEventDocument::ATokenTransfer(e) => Some(&e.tokenAddress),
    MoneyMarketEventDocument::DebtTokenMint(e) => Some(&e.tokenAddress),
    MoneyMarketEventDocument::DebtTokenBurn(e) => Some(&e.tokenAddress),
    _ => None,
  }
}

/// Extracted data from an event for processing
struct EventData {
  value: u128,
  balance_increase: u128,
  index: u128,
  block_number: u64,
  event_type: EventType,
  event_name: &'static str,
  /// The sign to apply when updating balance (+1 for increase, -1 for decrease)
  balance_sign: i128,
  /// Additional debug info for transfers
  transfer_info: Option<(String, String)>,
}

/// Extracts data from an event for processing
/// Returns None if the user is not involved in the event
fn extract_event_data(
  event: &MoneyMarketEventDocument,
  user_lower: &str,
  last_known_index: u128,
) -> Result<Option<EventData>, Box<dyn std::error::Error>> {
  match event {
    MoneyMarketEventDocument::ATokenMint(e) => {
      if e.onBehalfOf.to_lowercase() != user_lower {
        return Ok(None);
      }
      Ok(Some(EventData {
        value: decimal128_to_u128(e.value)?,
        balance_increase: decimal128_to_u128(e.balanceIncrease)?,
        index: decimal128_to_u128(e.index)?,
        block_number: e.common.blockNumber,
        event_type: EventType::Mint,
        event_name: "a-token-mint",
        balance_sign: 1,
        transfer_info: None,
      }))
    }
    MoneyMarketEventDocument::ATokenBurn(e) => {
      if e.from.to_lowercase() != user_lower {
        return Ok(None);
      }
      Ok(Some(EventData {
        value: decimal128_to_u128(e.value)?,
        balance_increase: decimal128_to_u128(e.balanceIncrease)?,
        index: decimal128_to_u128(e.index)?,
        block_number: e.common.blockNumber,
        event_type: EventType::Burn,
        event_name: "a-token-burn",
        balance_sign: -1,
        transfer_info: None,
      }))
    }
    MoneyMarketEventDocument::ATokenTransfer(e) => {
      let is_sender = e.from.to_lowercase() == user_lower;
      let is_recipient = e.to.to_lowercase() == user_lower;

      if !is_sender && !is_recipient {
        return Ok(None);
      }

      let balance_sign = if is_recipient { 1 } else { -1 };

      Ok(Some(EventData {
        value: decimal128_to_u128(e.value)?,
        balance_increase: 0,
        index: last_known_index,
        block_number: e.common.blockNumber,
        event_type: EventType::Transfer,
        event_name: "a-token-transfer",
        balance_sign,
        transfer_info: Some((e.from.clone(), e.to.clone())),
      }))
    }
    MoneyMarketEventDocument::DebtTokenMint(e) => {
      if e.onBehalfOf.to_lowercase() != user_lower {
        return Ok(None);
      }
      Ok(Some(EventData {
        value: decimal128_to_u128(e.value)?,
        balance_increase: decimal128_to_u128(e.balanceIncrease)?,
        index: decimal128_to_u128(e.index)?,
        block_number: e.common.blockNumber,
        event_type: EventType::Mint,
        event_name: "debt-token-mint",
        balance_sign: 1,
        transfer_info: None,
      }))
    }
    MoneyMarketEventDocument::DebtTokenBurn(e) => {
      if e.from.to_lowercase() != user_lower {
        return Ok(None);
      }
      Ok(Some(EventData {
        value: decimal128_to_u128(e.value)?,
        balance_increase: decimal128_to_u128(e.balanceIncrease)?,
        index: decimal128_to_u128(e.index)?,
        block_number: e.common.blockNumber,
        event_type: EventType::Burn,
        event_name: "debt-token-burn",
        balance_sign: -1,
        transfer_info: None,
      }))
    }
    _ => Ok(None),
  }
}

/// Prints debug information for an event
fn print_event_debug(
  idx: usize,
  event_data: &EventData,
  event_scaled: u128,
  scaled_before: i128,
  scaled_after: i128,
  real_before: i128,
  real_after: i128,
  verbose: bool,
) {
  if !verbose {
    return;
  }
  println!("{:03}. {}", idx + 1, event_data.event_name);
  println!("     Block: {}", event_data.block_number);

  if let Some((from, to)) = &event_data.transfer_info {
    println!("     From: {}", from);
    println!("     To: {}", to);
  }

  println!("     Value: {}", event_data.value);

  if event_data.event_type != EventType::Transfer {
    println!("     Balance Increase: {}", event_data.balance_increase);
    println!("     Index: {}", event_data.index);
  } else {
    println!("     Index (last known): {}", event_data.index);
  }

  println!("     Scaled: {}", event_scaled);
  println!("     Scaled Balance: {} → {}", scaled_before, scaled_after);
  println!("     Real Balance:   {} → {}\n", real_before, real_after);
}

/// Determines if a transfer event should be skipped (involves zero address)
fn should_skip_transfer_event(event: &MoneyMarketEventDocument) -> bool {
  match event {
    MoneyMarketEventDocument::ATokenTransfer(e) => {
      e.to.to_lowercase() == ZERO_ADDRESS.to_lowercase()
        || e.from.to_lowercase() == ZERO_ADDRESS.to_lowercase()
    }
    _ => false,
  }
}

/// Calculates the scaled balance from event parameters
/// Based on the AAVE V3 protocol formulas:
/// - Mint: (value - balanceIncrease) * RAY / index
/// - Burn: (value + balanceIncrease) * RAY / index
/// - Transfer: value * RAY / index
fn calculate_scaled_balance(
  value: u128,
  index: u128,
  balance_increase: u128,
  event_type: EventType,
  last_known_index: u128,
) -> Result<u128, Box<dyn std::error::Error>> {
  let big_value = U256::from(value);
  let big_index = if event_type == EventType::Transfer {
    U256::from(last_known_index)
  } else {
    U256::from(index)
  };
  let big_balance_increase = U256::from(balance_increase);
  let big_ray = U256::from(RAY);

  let result = match event_type {
    EventType::Mint => {
      // (value - balanceIncrease) * RAY / index
      let adjusted_value = big_value
        .checked_sub(big_balance_increase)
        .ok_or("Underflow in mint calculation")?;
      let scaled = adjusted_value
        .checked_mul(big_ray)
        .ok_or("Overflow in mint multiplication")?;
      scaled.checked_div(big_index).ok_or("Division by zero")?
    }
    EventType::Burn => {
      // (value + balanceIncrease) * RAY / index
      let adjusted_value = big_value
        .checked_add(big_balance_increase)
        .ok_or("Overflow in burn addition")?;
      let scaled = adjusted_value
        .checked_mul(big_ray)
        .ok_or("Overflow in burn multiplication")?;
      scaled.checked_div(big_index).ok_or("Division by zero")?
    }
    EventType::Transfer => {
      // value * RAY / index
      let scaled = big_value
        .checked_mul(big_ray)
        .ok_or("Overflow in transfer multiplication")?;
      scaled.checked_div(big_index).ok_or("Division by zero")?
    }
  };

  result
    .try_into()
    .map_err(|_| "Failed to convert result to u128".into())
}

/// Converts scaled balance to real balance using the current index
/// Formula: scaledBalance * currentIndex / RAY
fn convert_scaled_to_real_balance(
  scaled_balance: u128,
  index: u128,
) -> Result<u128, Box<dyn std::error::Error>> {
  let big_scaled = U256::from(scaled_balance);
  let big_index = U256::from(index);
  let big_ray = U256::from(RAY);

  let result = big_scaled
    .checked_mul(big_index)
    .ok_or("Overflow in conversion")?;
  let real_balance = result.checked_div(big_ray).ok_or("Division by zero")?;

  real_balance
    .try_into()
    .map_err(|_| "Failed to convert real balance to u128".into())
}

/// Helper to convert Decimal128 to u128
fn decimal128_to_u128(d: Decimal128) -> Result<u128, Box<dyn std::error::Error>> {
  d.to_string()
    .parse::<u128>()
    .map_err(|e| format!("Failed to parse Decimal128 to u128: {}", e).into())
}

/// Processes events for a specific user and token to calculate scaled balance
/// Returns the final scaled balance and the real balance using the current index
pub fn process_user_token_events(
  events: &[MoneyMarketEventDocument],
  user_address: &str,
  token_address: &str,
  current_index: u128,
  verbose: bool,
) -> Result<BalanceResult, Box<dyn std::error::Error>> {
  let user_lower = user_address.to_lowercase();
  let token_lower = token_address.to_lowercase();

  let mut scaled_balance: i128 = 0;
  let mut real_balance: i128 = 0;
  let mut last_index = RAY;
  let mut last_event_block: u64 = 0;

  if verbose {
    println!("\n=== Processing Events for User and Token ===");
    println!("User: {}", user_address);
    println!("Token: {}", token_address);
    println!("Current Index: {}\n", current_index);
  }

  for (idx, event) in events.iter().enumerate() {
    // Skip events not related to our token
    if let Some(event_token_addr) = get_event_token_address(event) {
      if event_token_addr.to_lowercase() != token_lower {
        continue;
      }
    } else {
      continue;
    }

    // Skip transfer events involving zero address
    if should_skip_transfer_event(event) {
      if verbose {
        println!(
          "{:03}. Skipping transfer event at block {}",
          idx + 1,
          event.block_number()
        );
      }
      continue;
    }

    // Extract event data - skip if user is not involved
    let event_data = match extract_event_data(event, &user_lower, last_index)? {
      Some(data) => data,
      None => continue,
    };

    // Update last_index for non-transfer events
    // NOTE: For transfer events, we use the last known index, accuracy of this is not 100%
    // ideally we would fetch the index at the block of the transfer, but this is a reasonable
    // approximation and a good enough trade-off for this tool.
    if event_data.event_type != EventType::Transfer {
      last_index = event_data.index;
    }

    // Calculate scaled balance for this event
    let event_scaled = calculate_scaled_balance(
      event_data.value,
      event_data.index,
      event_data.balance_increase,
      event_data.event_type,
      last_index,
    )?;

    // Store before values for debug printing
    let scaled_before = scaled_balance;
    let real_before = real_balance;

    // Update balances
    scaled_balance += event_data.balance_sign * event_scaled as i128;
    real_balance += event_data.balance_sign * event_data.value as i128;
    last_event_block = event_data.block_number;

    // Print debug information
    print_event_debug(
      idx,
      &event_data,
      event_scaled,
      scaled_before,
      scaled_balance,
      real_before,
      real_balance,
      verbose,
    );
  }

  // Convert final scaled balance to real balance using the index from the last event
  // This is important: we should use last_index (from the last event) not current_index
  // because we're calculating the balance as it was at the last event
  let final_scaled_balance = if scaled_balance < 0 {
    0
  } else {
    scaled_balance as u128
  };
  let final_real_balance = if real_balance < 0 {
    0
  } else {
    real_balance as u128
  };
  let calculated_real_from_scaled =
    convert_scaled_to_real_balance(final_scaled_balance, last_index)?;

  if verbose {
    println!("=== Final Results ===");
    println!("Final Scaled Balance: {}", final_scaled_balance);
    println!(
      "Final Real Balance (from scaled): {}",
      calculated_real_from_scaled
    );
    println!("Final Real Balance (direct sum): {}", final_real_balance);
    println!("Last Event Block: {}", last_event_block);
  }

  Ok(BalanceResult {
    scaled_balance: final_scaled_balance,
    real_balance: calculated_real_from_scaled,
    last_index,
    last_event_block,
  })
}
