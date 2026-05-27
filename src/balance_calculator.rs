use crate::output;
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

/// Event type for calculation purposes.
///
/// `BalanceTransfer` represents Aave's `BalanceTransfer(from, to, amount.rayDiv(index), index)`
/// event — the `value` field is ALREADY the scaled amount, so we apply it directly without
/// dividing by index. We deliberately do not process the paired `a-token-transfer` (ERC-20
/// Transfer) event for real user-to-user transfers: that's the same physical transfer in
/// real units, and using it would either double-count or force us back onto a stale
/// `last_known_index` approximation.
#[derive(Debug, Clone, Copy, PartialEq)]
enum EventType {
  Mint,
  Burn,
  BalanceTransfer,
}

/// Gets the token address from an event
fn get_event_token_address(event: &MoneyMarketEventDocument) -> Option<&str> {
  match event {
    MoneyMarketEventDocument::ATokenMint(e) => Some(&e.tokenAddress),
    MoneyMarketEventDocument::ATokenBurn(e) => Some(&e.tokenAddress),
    MoneyMarketEventDocument::ATokenBalanceTransfer(e) => Some(&e.tokenAddress),
    MoneyMarketEventDocument::DebtTokenMint(e) => Some(&e.tokenAddress),
    MoneyMarketEventDocument::DebtTokenBurn(e) => Some(&e.tokenAddress),
    _ => None,
  }
}

/// Extracted data from an event for processing.
///
/// `value` semantics depend on `event_type`:
/// - `Mint`/`Burn`: the real token amount (Aave's `Mint`/`Burn` event `value` field).
/// - `BalanceTransfer`: the scaled amount (Aave emits `amount.rayDiv(index)` directly).
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

/// Extracts data from an event for processing.
/// Returns None if the user is not involved in the event.
///
/// `a-token-transfer` is intentionally ignored: every real user-to-user aToken transfer
/// emits both a `BalanceTransfer` (carrying the scaled amount + the exact block index)
/// and a paired ERC-20 `Transfer` (carrying the real amount). We process only the former
/// — it's the lossless signal. The latter would either double-count or fall back to a
/// stale `last_known_index` approximation. (Transfer events with zero from/to are
/// mint/burn shadows already handled by the Mint/Burn arms.)
fn extract_event_data(
  event: &MoneyMarketEventDocument,
  user_lower: &str,
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
    MoneyMarketEventDocument::ATokenBalanceTransfer(e) => {
      let is_sender = e.from.to_lowercase() == user_lower;
      let is_recipient = e.to.to_lowercase() == user_lower;

      // Skip if uninvolved, OR a self-transfer (net effect on this user's balance is zero —
      // crediting +value once would be wrong by `value` scaled units).
      if (!is_sender && !is_recipient) || (is_sender && is_recipient) {
        return Ok(None);
      }

      let balance_sign = if is_recipient { 1 } else { -1 };

      Ok(Some(EventData {
        value: decimal128_to_u128(e.value)?,
        balance_increase: 0,
        index: decimal128_to_u128(e.index)?,
        block_number: e.common.blockNumber,
        event_type: EventType::BalanceTransfer,
        event_name: "a-token-balance-transfer",
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

/// Extracts the index from any index-carrying event regardless of which user it belongs to.
/// Used to keep `last_index` current for the final scaled→real conversion at the end of
/// the replay.
fn get_event_index(event: &MoneyMarketEventDocument) -> Option<u128> {
  match event {
    MoneyMarketEventDocument::ATokenMint(e) => decimal128_to_u128(e.index).ok(),
    MoneyMarketEventDocument::ATokenBurn(e) => decimal128_to_u128(e.index).ok(),
    MoneyMarketEventDocument::ATokenBalanceTransfer(e) => decimal128_to_u128(e.index).ok(),
    MoneyMarketEventDocument::DebtTokenMint(e) => decimal128_to_u128(e.index).ok(),
    MoneyMarketEventDocument::DebtTokenBurn(e) => decimal128_to_u128(e.index).ok(),
    _ => None,
  }
}

/// Prints debug information for an event
#[allow(clippy::too_many_arguments)]
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
  output!("{:03}. {}", idx + 1, event_data.event_name);
  output!("     Block: {}", event_data.block_number);

  if let Some((from, to)) = &event_data.transfer_info {
    output!("     From: {}", from);
    output!("     To: {}", to);
  }

  if event_data.event_type == EventType::BalanceTransfer {
    output!("     Scaled Amount: {}", event_data.value);
    output!("     Index: {}", event_data.index);
  } else {
    output!("     Value: {}", event_data.value);
    output!("     Balance Increase: {}", event_data.balance_increase);
    output!("     Index: {}", event_data.index);
  }

  output!("     Scaled: {}", event_scaled);
  output!("     Scaled Balance: {} → {}", scaled_before, scaled_after);
  output!("     Real Balance:   {} → {}\n", real_before, real_after);
}

/// Determines if a transfer event should be skipped (involves zero address)
pub fn should_skip_transfer_event(event: &MoneyMarketEventDocument) -> bool {
  match event {
    MoneyMarketEventDocument::ATokenTransfer(e) => {
      e.to.to_lowercase() == ZERO_ADDRESS.to_lowercase()
        || e.from.to_lowercase() == ZERO_ADDRESS.to_lowercase()
    }
    _ => false,
  }
}

/// Calculates the scaled balance delta from event parameters.
/// Based on the AAVE V3 protocol formulas:
/// - Mint: (value - balanceIncrease) * RAY / index
/// - Burn: (value + balanceIncrease) * RAY / index
/// - BalanceTransfer: value (already scaled — Aave emits `amount.rayDiv(index)`)
fn calculate_scaled_balance(
  value: u128,
  index: u128,
  balance_increase: u128,
  event_type: EventType,
) -> Result<u128, Box<dyn std::error::Error>> {
  let big_value = U256::from(value);
  let big_index = U256::from(index);
  let big_balance_increase = U256::from(balance_increase);
  let big_ray = U256::from(RAY);

  let result = match event_type {
    EventType::Mint => {
      // (value - balanceIncrease) * RAY / index
      // Use saturating_sub: when value < balanceIncrease (pure interest accrual
      // or rounding), the net new scaled amount is 0.
      let adjusted_value = big_value.saturating_sub(big_balance_increase);
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
    EventType::BalanceTransfer => big_value,
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
    output!("\n=== Processing Events for User and Token ===");
    output!("User: {}", user_address);
    output!("Token: {}", token_address);
    output!("Current Index: {}\n", current_index);
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

    // Update last_index from any index-carrying event for this token (regardless of
    // user). This keeps last_index current for the final scaled→real conversion below.
    // (We already know the event matches our token from the filter above.)
    if let Some(event_index) = get_event_index(event) {
      last_index = event_index;
    }

    // `should_skip_transfer_event` is intentionally NOT called here: every event reaching
    // this point already has a non-None `get_event_token_address` arm (Mint, Burn,
    // ATokenBalanceTransfer, DebtTokenMint, DebtTokenBurn). None match
    // `should_skip_transfer_event`'s ATokenTransfer arm, so the call would be unreachable.
    // The helper remains `pub` for use by `handle_inspect_user_position`, which iterates
    // a different event stream that still includes ATokenTransfer.

    // Extract event data - skip if user is not involved
    let event_data = match extract_event_data(event, &user_lower)? {
      Some(data) => data,
      None => continue,
    };

    // Calculate scaled balance for this event
    let event_scaled = calculate_scaled_balance(
      event_data.value,
      event_data.index,
      event_data.balance_increase,
      event_data.event_type,
    )?;

    // Store before values for debug printing
    let scaled_before = scaled_balance;
    let real_before = real_balance;

    // Update balances. For BalanceTransfer, event_data.value IS the scaled amount, so the
    // running real_balance counter (verbose display only) takes the real-equivalent via
    // the event's own index. For Mint/Burn, value is already the real token amount.
    scaled_balance += event_data.balance_sign * event_scaled as i128;
    let real_delta = if event_data.event_type == EventType::BalanceTransfer {
      convert_scaled_to_real_balance(event_data.value, event_data.index)?
    } else {
      event_data.value
    };
    real_balance += event_data.balance_sign * real_delta as i128;
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
    output!("=== Final Results ===");
    output!("Final Scaled Balance: {}", final_scaled_balance);
    output!(
      "Final Real Balance (from scaled): {}",
      calculated_real_from_scaled
    );
    output!("Final Real Balance (direct sum): {}", final_real_balance);
    output!("Last Event Block: {}", last_event_block);
  }

  Ok(BalanceResult {
    scaled_balance: final_scaled_balance,
    real_balance: calculated_real_from_scaled,
    last_index,
    last_event_block,
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::models::{ATokenBalanceTransferEvent, ATokenMintEvent, ATokenTransferEvent, CommonFields};
  use mongodb::bson::oid::ObjectId;

  const TOKEN: &str = "0x5c50cf875aaaaa0bbbbb1111111111111111aaaa";
  const ALICE: &str = "0xaff2edb3057ed6f9c1da6c930b8dddf2bee573a5";
  const BOB: &str = "0x1234567890abcdef1234567890abcdef12345678";
  // 1.05 * RAY (typical Aave liquidity index a few percent above RAY).
  const INDEX_1_05: u128 = 1_050_000_000_000_000_000_000_000_000;
  // 1.10 * RAY (later block, more interest accrued).
  const INDEX_1_10: u128 = 1_100_000_000_000_000_000_000_000_000;

  fn common(block: u64, log_idx: i64) -> CommonFields {
    CommonFields {
      id: ObjectId::new(),
      txHash: format!("0x{:064x}", block),
      logIndex: log_idx,
      chainId: 1,
      blockNumber: block,
      version: 0,
    }
  }

  fn dec(n: u128) -> Decimal128 {
    n.to_string().parse().unwrap()
  }

  fn balance_transfer(
    block: u64,
    log_idx: i64,
    from: &str,
    to: &str,
    scaled_value: u128,
    index: u128,
  ) -> MoneyMarketEventDocument {
    MoneyMarketEventDocument::ATokenBalanceTransfer(ATokenBalanceTransferEvent {
      common: common(block, log_idx),
      tokenAddress: TOKEN.to_string(),
      from: from.to_string(),
      to: to.to_string(),
      value: dec(scaled_value),
      index: dec(index),
    })
  }

  fn a_token_transfer(
    block: u64,
    log_idx: i64,
    from: &str,
    to: &str,
    real_value: u128,
  ) -> MoneyMarketEventDocument {
    MoneyMarketEventDocument::ATokenTransfer(ATokenTransferEvent {
      common: common(block, log_idx),
      tokenAddress: TOKEN.to_string(),
      from: from.to_string(),
      to: to.to_string(),
      value: dec(real_value),
    })
  }

  fn mint(
    block: u64,
    log_idx: i64,
    on_behalf_of: &str,
    real_value: u128,
    balance_increase: u128,
    index: u128,
  ) -> MoneyMarketEventDocument {
    MoneyMarketEventDocument::ATokenMint(ATokenMintEvent {
      common: common(block, log_idx),
      tokenAddress: TOKEN.to_string(),
      caller: BOB.to_string(),
      onBehalfOf: on_behalf_of.to_string(),
      value: dec(real_value),
      balanceIncrease: dec(balance_increase),
      index: dec(index),
    })
  }

  /// Regression for sodaAVAX bug (sodax-backend issue #577): a user whose only events on
  /// a token are a-token-balance-transfer must end up with the scaled amount applied,
  /// not silently dropped (which collapsed to DB Events Scaled = 0 in the canary run).
  #[test]
  fn transfer_only_user_credits_scaled_amount_from_balance_transfer() {
    let events = vec![balance_transfer(100, 0, BOB, ALICE, 100, INDEX_1_05)];
    let result = process_user_token_events(&events, ALICE, TOKEN, RAY, false).unwrap();
    assert_eq!(result.scaled_balance, 100);
  }

  /// A user that receives N scaled then sends the same N scaled out at a later (higher)
  /// liquidity index ends at exactly 0 scaled. The pre-fix code, working from
  /// a-token-transfer (real-amount) events with `last_known_index` stuck at RAY because
  /// the user had no Mint/Burn, produced a net-negative balance that got clamped to 0.
  /// Verify the new path nets cleanly without relying on the clamp.
  #[test]
  fn balance_transfer_in_then_out_nets_to_zero_across_index_growth() {
    let events = vec![
      balance_transfer(100, 0, BOB, ALICE, 100, INDEX_1_05),
      balance_transfer(200, 0, ALICE, BOB, 100, INDEX_1_10),
    ];
    let result = process_user_token_events(&events, ALICE, TOKEN, RAY, false).unwrap();
    assert_eq!(result.scaled_balance, 0);
  }

  /// Every real Aave V3 aToken transfer emits BOTH a-token-balance-transfer (scaled +
  /// index) and a paired a-token-transfer (real amount). The replay must apply the
  /// balance-transfer once and ignore the paired ERC-20 transfer — otherwise the same
  /// physical transfer would be counted twice (or fall back to a stale-index estimate).
  #[test]
  fn paired_a_token_transfer_is_not_double_counted() {
    let events = vec![
      balance_transfer(100, 0, BOB, ALICE, 100, INDEX_1_05),
      // Paired ERC-20 view: 100 scaled at index 1.05 = 105 real.
      a_token_transfer(100, 1, BOB, ALICE, 105),
    ];
    let result = process_user_token_events(&events, ALICE, TOKEN, RAY, false).unwrap();
    assert_eq!(result.scaled_balance, 100);
  }

  /// A self-transfer's net effect on the holder's scaled balance is zero (Aave's
  /// `super._transfer(sender, recipient, scaled)` is a no-op when sender == recipient).
  /// Crediting +scaled once because the user matched the `to` field would be wrong.
  #[test]
  fn self_transfer_is_noop() {
    let events = vec![balance_transfer(100, 0, ALICE, ALICE, 100, INDEX_1_05)];
    let result = process_user_token_events(&events, ALICE, TOKEN, RAY, false).unwrap();
    assert_eq!(result.scaled_balance, 0);
  }

  /// last_index is what drives the final scaled→real conversion in BalanceResult.
  /// a-token-balance-transfer events carry the exact block index and must update
  /// last_index — otherwise the returned real_balance would use RAY (1.0) for users
  /// with no Mint/Burn events.
  #[test]
  fn balance_transfer_updates_last_index_for_final_real_conversion() {
    let events = vec![balance_transfer(100, 0, BOB, ALICE, 100, INDEX_1_05)];
    let result = process_user_token_events(&events, ALICE, TOKEN, RAY, false).unwrap();
    assert_eq!(result.last_index, INDEX_1_05);
    // 100 scaled * 1.05 index = 105 real.
    assert_eq!(result.real_balance, 105);
  }

  /// Mint events still produce the correct scaled delta after the refactor that dropped
  /// the `last_known_index` parameter from calculate_scaled_balance. Guards against the
  /// signature change breaking the supply path.
  #[test]
  fn mint_event_still_applies_correct_scaled_delta() {
    // value=105, balance_increase=0, index=1.05 → (105 - 0) * RAY / 1.05 RAY = 100 scaled.
    let events = vec![mint(100, 0, ALICE, 105, 0, INDEX_1_05)];
    let result = process_user_token_events(&events, ALICE, TOKEN, RAY, false).unwrap();
    assert_eq!(result.scaled_balance, 100);
  }
}
