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

/// Determines the event type for calculation purposes
#[allow(dead_code)]
fn get_event_type_for_calculation(event: &MoneyMarketEventDocument) -> EventType {
  match event {
    MoneyMarketEventDocument::ATokenMint(_) | MoneyMarketEventDocument::DebtTokenMint(_) => {
      EventType::Mint
    }
    MoneyMarketEventDocument::ATokenBurn(_) | MoneyMarketEventDocument::DebtTokenBurn(_) => {
      EventType::Burn
    }
    MoneyMarketEventDocument::ATokenTransfer(_) => EventType::Transfer,
    _ => EventType::Transfer, // Fallback
  }
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
) -> Result<BalanceResult, Box<dyn std::error::Error>> {
  let user_lower = user_address.to_lowercase();
  let token_lower = token_address.to_lowercase();

  let mut scaled_balance: i128 = 0;
  let mut real_balance: i128 = 0;
  let mut last_index = RAY;
  let mut last_event_block: u64 = 0;

  println!("\n=== Processing Events for User and Token ===");
  println!("User: {}", user_address);
  println!("Token: {}", token_address);
  println!("Current Index: {}\n", current_index);

  for (idx, event) in events.iter().enumerate() {
    // Skip events not related to our token
    let event_token = match event {
      MoneyMarketEventDocument::ATokenMint(e) => Some(&e.tokenAddress),
      MoneyMarketEventDocument::ATokenBurn(e) => Some(&e.tokenAddress),
      MoneyMarketEventDocument::ATokenTransfer(e) => Some(&e.tokenAddress),
      MoneyMarketEventDocument::DebtTokenMint(e) => Some(&e.tokenAddress),
      MoneyMarketEventDocument::DebtTokenBurn(e) => Some(&e.tokenAddress),
      _ => None,
    };

    if let Some(et) = event_token {
      if et.to_lowercase() != token_lower {
        continue;
      }
    } else {
      continue;
    }

    // Skip transfer events involving zero address
    if should_skip_transfer_event(event) {
      println!(
        "{:03}. Skipping transfer event at block {}",
        idx + 1,
        event.block_number()
      );
      continue;
    }

    match event {
      MoneyMarketEventDocument::ATokenMint(e) => {
        let is_user_event = e.onBehalfOf.to_lowercase() == user_lower;
        if !is_user_event {
          continue;
        }

        let value = decimal128_to_u128(e.value)?;
        let balance_increase = decimal128_to_u128(e.balanceIncrease)?;
        let index = decimal128_to_u128(e.index)?;
        last_index = index;

        let event_scaled = calculate_scaled_balance(
          value,
          index,
          balance_increase,
          EventType::Mint,
          last_index,
        )?;

        let scaled_before = scaled_balance;
        let real_before = real_balance;

        scaled_balance += event_scaled as i128;
        real_balance += value as i128;
        last_event_block = e.common.blockNumber;

        println!("{:03}. a-token-mint", idx + 1);
        println!("     Block: {}", e.common.blockNumber);
        println!("     Value: {}", value);
        println!("     Balance Increase: {}", balance_increase);
        println!("     Index: {}", index);
        println!("     Scaled: {}", event_scaled);
        println!(
          "     Scaled Balance: {} → {}",
          scaled_before, scaled_balance
        );
        println!(
          "     Real Balance:   {} → {}\n",
          real_before, real_balance
        );
      }
      MoneyMarketEventDocument::ATokenBurn(e) => {
        let is_user_event = e.from.to_lowercase() == user_lower;
        if !is_user_event {
          continue;
        }

        let value = decimal128_to_u128(e.value)?;
        let balance_increase = decimal128_to_u128(e.balanceIncrease)?;
        let index = decimal128_to_u128(e.index)?;

        let event_scaled =
          calculate_scaled_balance(value, index, balance_increase, EventType::Burn, last_index)?;

        let scaled_before = scaled_balance;
        let real_before = real_balance;

        scaled_balance -= event_scaled as i128;
        real_balance -= value as i128;
        last_event_block = e.common.blockNumber;

        println!("{:03}. a-token-burn", idx + 1);
        println!("     Block: {}", e.common.blockNumber);
        println!("     Value: {}", value);
        println!("     Balance Increase: {}", balance_increase);
        println!("     Index: {}", index);
        println!("     Scaled: {}", event_scaled);
        println!(
          "     Scaled Balance: {} → {}",
          scaled_before, scaled_balance
        );
        println!(
          "     Real Balance:   {} → {}\n",
          real_before, real_balance
        );
      }
      MoneyMarketEventDocument::ATokenTransfer(e) => {
        let is_sender = e.from.to_lowercase() == user_lower;
        let is_recipient = e.to.to_lowercase() == user_lower;

        if !is_sender && !is_recipient {
          continue;
        }

        let value = decimal128_to_u128(e.value)?;

        let event_scaled =
          calculate_scaled_balance(value, last_index, 0, EventType::Transfer, last_index)?;

        let scaled_before = scaled_balance;
        let real_before = real_balance;

        if is_recipient {
          scaled_balance += event_scaled as i128;
          real_balance += value as i128;
        } else if is_sender {
          scaled_balance -= event_scaled as i128;
          real_balance -= value as i128;
        }
        last_event_block = e.common.blockNumber;

        println!("{:03}. a-token-transfer", idx + 1);
        println!("     Block: {}", e.common.blockNumber);
        println!("     From: {}", e.from);
        println!("     To: {}", e.to);
        println!("     Value: {}", value);
        println!("     Index (last known): {}", last_index);
        println!("     Scaled: {}", event_scaled);
        println!(
          "     Scaled Balance: {} → {}",
          scaled_before, scaled_balance
        );
        println!(
          "     Real Balance:   {} → {}\n",
          real_before, real_balance
        );
      }
      MoneyMarketEventDocument::DebtTokenMint(e) => {
        let is_user_event = e.onBehalfOf.to_lowercase() == user_lower;
        if !is_user_event {
          continue;
        }

        let value = decimal128_to_u128(e.value)?;
        let balance_increase = decimal128_to_u128(e.balanceIncrease)?;
        let index = decimal128_to_u128(e.index)?;
        last_index = index;

        let event_scaled = calculate_scaled_balance(
          value,
          index,
          balance_increase,
          EventType::Mint,
          last_index,
        )?;

        let scaled_before = scaled_balance;
        let real_before = real_balance;

        scaled_balance += event_scaled as i128;
        real_balance += value as i128;
        last_event_block = e.common.blockNumber;

        println!("{:03}. debt-token-mint", idx + 1);
        println!("     Block: {}", e.common.blockNumber);
        println!("     Value: {}", value);
        println!("     Balance Increase: {}", balance_increase);
        println!("     Index: {}", index);
        println!("     Scaled: {}", event_scaled);
        println!(
          "     Scaled Balance: {} → {}",
          scaled_before, scaled_balance
        );
        println!(
          "     Real Balance:   {} → {}\n",
          real_before, real_balance
        );
      }
      MoneyMarketEventDocument::DebtTokenBurn(e) => {
        let is_user_event = e.from.to_lowercase() == user_lower;
        if !is_user_event {
          continue;
        }

        let value = decimal128_to_u128(e.value)?;
        let balance_increase = decimal128_to_u128(e.balanceIncrease)?;
        let index = decimal128_to_u128(e.index)?;

        let event_scaled =
          calculate_scaled_balance(value, index, balance_increase, EventType::Burn, last_index)?;

        let scaled_before = scaled_balance;
        let real_before = real_balance;

        scaled_balance -= event_scaled as i128;
        real_balance -= value as i128;
        last_event_block = e.common.blockNumber;

        println!("{:03}. debt-token-burn", idx + 1);
        println!("     Block: {}", e.common.blockNumber);
        println!("     Value: {}", value);
        println!("     Balance Increase: {}", balance_increase);
        println!("     Index: {}", index);
        println!("     Scaled: {}", event_scaled);
        println!(
          "     Scaled Balance: {} → {}",
          scaled_before, scaled_balance
        );
        println!(
          "     Real Balance:   {} → {}\n",
          real_before, real_balance
        );
      }
      _ => {}
    }
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

  println!("=== Final Results ===");
  println!("Final Scaled Balance: {}", final_scaled_balance);
  println!(
    "Final Real Balance (from scaled): {}",
    calculated_real_from_scaled
  );
  println!(
    "Final Real Balance (direct sum): {}",
    final_real_balance
  );
  println!("Last Event Block: {}", last_event_block);

  Ok(BalanceResult {
    scaled_balance: final_scaled_balance,
    real_balance: calculated_real_from_scaled,
    last_index,
    last_event_block,
  })
}

