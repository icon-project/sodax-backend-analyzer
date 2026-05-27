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
///
/// `PhantomMintFromBurn` is the Aave `_burnScaled` "interest exceeds amount" branch: when
/// a user repays/withdraws an amount smaller than the interest accrued since their last
/// touch, the chain still calls `super._burn(user, amountScaled)` — so the scaled balance
/// *decreases* by `amount.rayDiv(index)` — but the emitted event is `Mint(value, bi)`
/// with `value = balanceIncrease - amount` (i.e. `value < balanceIncrease`). The
/// `_mintScaled` path always emits `value = amount + balanceIncrease >= balanceIncrease`,
/// so the `value < balanceIncrease` check is a clean discriminator between the two paths.
/// Without this variant the analyzer treats those phantom mints as no-ops (the old
/// `saturating_sub` clamped them to zero) and over-counts the user's balance by the
/// scaled equivalent of every missed debit — empirically the root cause of the 280×–9000×
/// `[FAIL]` rows on the 2026-05-27 canary.
#[derive(Debug, Clone, Copy, PartialEq)]
enum EventType {
  Mint,
  Burn,
  BalanceTransfer,
  PhantomMintFromBurn,
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

/// Discriminates between a normal Aave Mint (from `_mintScaled`) and the phantom Mint
/// emitted by `_burnScaled` when accrued interest exceeds the repaid/withdrawn amount.
/// See the `EventType::PhantomMintFromBurn` doc-block for the full rationale.
///
/// `_mintScaled` always emits `value = amount + balanceIncrease`, so `value > balanceIncrease`
/// whenever `amount > 0` (and `amountScaled != 0` is a require). `_burnScaled`'s mint
/// branch emits `value = balanceIncrease - amount`, so `value < balanceIncrease` whenever
/// `amount > 0`. The strict-less-than check therefore cleanly separates the two paths.
fn mint_classification(value: u128, balance_increase: u128) -> (EventType, i128) {
  if value < balance_increase {
    (EventType::PhantomMintFromBurn, -1)
  } else {
    (EventType::Mint, 1)
  }
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
      let value = decimal128_to_u128(e.value)?;
      let balance_increase = decimal128_to_u128(e.balanceIncrease)?;
      let (event_type, balance_sign) = mint_classification(value, balance_increase);
      Ok(Some(EventData {
        value,
        balance_increase,
        index: decimal128_to_u128(e.index)?,
        block_number: e.common.blockNumber,
        event_type,
        event_name: "a-token-mint",
        balance_sign,
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
      let value = decimal128_to_u128(e.value)?;
      let balance_increase = decimal128_to_u128(e.balanceIncrease)?;
      let (event_type, balance_sign) = mint_classification(value, balance_increase);
      Ok(Some(EventData {
        value,
        balance_increase,
        index: decimal128_to_u128(e.index)?,
        block_number: e.common.blockNumber,
        event_type,
        event_name: "debt-token-mint",
        balance_sign,
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
  let phantom_tag = if event_data.event_type == EventType::PhantomMintFromBurn {
    " (phantom mint from _burnScaled — value<bi, applied as debit)"
  } else {
    ""
  };
  output!("{:03}. {}{}", idx + 1, event_data.event_name, phantom_tag);
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
/// - Mint:                (value - balanceIncrease) * RAY / index
/// - Burn:                (value + balanceIncrease) * RAY / index
/// - BalanceTransfer:     value (already scaled — Aave emits `amount.rayDiv(index)`)
/// - PhantomMintFromBurn: (balanceIncrease - value) * RAY / index
///   The on-chain effect is a burn of `amount.rayDiv(index)` where `amount = bi - value`.
///   The caller pairs this with `balance_sign = -1` so the returned magnitude is debited.
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
      // `mint_classification` routes `value < balanceIncrease` to PhantomMintFromBurn,
      // so by the time we land here `value >= balanceIncrease` is guaranteed. A bare
      // `checked_sub` is enough; the previous `saturating_sub` masked the phantom-mint
      // case as a no-op and was the root cause of the canary-flagged over-counts.
      let adjusted_value = big_value
        .checked_sub(big_balance_increase)
        .ok_or("Underflow in mint subtraction (value < balanceIncrease should route to PhantomMintFromBurn)")?;
      let scaled = adjusted_value
        .checked_mul(big_ray)
        .ok_or("Overflow in mint multiplication")?;
      scaled.checked_div(big_index).ok_or("Division by zero")?
    }
    EventType::PhantomMintFromBurn => {
      // (balanceIncrease - value) * RAY / index
      // Underflow is unreachable: `mint_classification` only assigns this variant when
      // `value < balanceIncrease`, but `checked_sub` is used defensively.
      let adjusted_value = big_balance_increase
        .checked_sub(big_value)
        .ok_or("Underflow in phantom-mint subtraction (value >= balanceIncrease should route to Mint)")?;
      let scaled = adjusted_value
        .checked_mul(big_ray)
        .ok_or("Overflow in phantom-mint multiplication")?;
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

    // Update balances. The running real_balance is verbose-display only; the authoritative
    // return value is derived from the final scaled_balance via convert_scaled_to_real_balance.
    //   - Mint:                value already = amount + balanceIncrease (real cashflow + interest)
    //   - Burn:                value already = amount - balanceIncrease (net real cashflow)
    //   - BalanceTransfer:     value is SCALED; convert to real via the event's own index
    //   - PhantomMintFromBurn: value = balanceIncrease - amount (the emitted phantom);
    //                          the *actual* real cashflow on the chain was a burn of `amount`,
    //                          so the real_delta is `balanceIncrease - value` (= amount).
    scaled_balance += event_data.balance_sign * event_scaled as i128;
    let real_delta = match event_data.event_type {
      EventType::BalanceTransfer => convert_scaled_to_real_balance(event_data.value, event_data.index)?,
      EventType::PhantomMintFromBurn => event_data.balance_increase - event_data.value,
      EventType::Mint | EventType::Burn => event_data.value,
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

  fn debt_mint(
    block: u64,
    log_idx: i64,
    on_behalf_of: &str,
    real_value: u128,
    balance_increase: u128,
    index: u128,
  ) -> MoneyMarketEventDocument {
    MoneyMarketEventDocument::DebtTokenMint(crate::models::DebtTokenMintEvent {
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

  // ----------------------------------------------------------
  // PhantomMintFromBurn: Aave's `_burnScaled` net-interest branch
  // ----------------------------------------------------------
  //
  // Regression set for sodax-backend issue #577 round 2 (the 3 remaining canary [FAIL]s
  // after the transfer-event fix). When a user repays/withdraws less than the interest
  // accrued since their last touch, Aave's `_burnScaled` still calls
  // `super._burn(user, amount.rayDiv(index))` — the scaled balance decreases — but the
  // emitted event is a Mint with `value = balanceIncrease - amount` (so `value < bi`).
  // The pre-fix `saturating_sub` clamped these to a no-op; the analyzer over-counted by
  // the entire scaled-burn equivalent of every missed phantom-mint event, exactly
  // matching the canary's 280×–9000× over-counts.

  /// Direct helper test: `value < balance_increase` routes to PhantomMintFromBurn (debit).
  /// `value >= balance_increase` routes to Mint (credit).
  #[test]
  fn mint_classification_discriminates_on_value_vs_balance_increase() {
    assert_eq!(mint_classification(20, 50), (EventType::PhantomMintFromBurn, -1));
    assert_eq!(mint_classification(50, 20), (EventType::Mint, 1));
    // Equality routes to Mint (compute path produces 0 scaled, which is correct: amount=0
    // can't actually be emitted by Aave's _mintScaled require, but routing must be defined).
    assert_eq!(mint_classification(30, 30), (EventType::Mint, 1));
  }

  /// Repro of the canary pattern at small scale: user supplies 100 scaled at index=1.0;
  /// time passes, index drifts to 1.5 (real balance becomes 150, interest accrued = 50);
  /// user repays 30. Aave emits Mint(value=20, bi=50, index=1.5) — the phantom signature.
  /// On-chain `super._burn(user, 30.rayDiv(1.5) = 20)` drops scaled from 100 to 80.
  ///
  /// Pre-fix: `saturating_sub(20, 50) = 0`, scaled_balance unchanged at 100 (off by +20).
  /// Post-fix: PhantomMintFromBurn applies `(50-20)*RAY/1.5RAY = 20`, debited → 80. ✓
  #[test]
  fn phantom_mint_from_burn_debits_scaled_balance() {
    let index_1_5: u128 = 1_500_000_000_000_000_000_000_000_000;
    let events = vec![
      mint(100, 0, ALICE, 100, 0, RAY),
      mint(200, 0, ALICE, 20, 50, index_1_5),
    ];
    let result = process_user_token_events(&events, ALICE, TOKEN, RAY, false).unwrap();
    assert_eq!(result.scaled_balance, 80);
  }

  /// Variable-debt parity: the same `_burnScaled` code path runs for debt tokens (on
  /// repays). The canary's worst offenders (bnUSDd, sodaUSDC borrow) were debt-token
  /// phantoms; this test asserts the routing through DebtTokenMint matches ATokenMint.
  #[test]
  fn phantom_mint_from_burn_works_for_debt_token() {
    let index_1_5: u128 = 1_500_000_000_000_000_000_000_000_000;
    let events = vec![
      debt_mint(100, 0, ALICE, 100, 0, RAY),
      debt_mint(200, 0, ALICE, 20, 50, index_1_5),
    ];
    let result = process_user_token_events(&events, ALICE, TOKEN, RAY, false).unwrap();
    assert_eq!(result.scaled_balance, 80);
  }

  /// Pre-fix smoke test: the `saturating_sub` would have silently absorbed every
  /// phantom-mint as 0, leaving the original scaled balance untouched across an arbitrary
  /// stream of phantoms. Post-fix, multiple phantoms accumulate as separate debits.
  /// Two phantoms each debiting 10 scaled (at index=1.0) drop the running total by 20.
  #[test]
  fn multiple_phantom_mints_accumulate_as_debits() {
    let events = vec![
      mint(100, 0, ALICE, 100, 0, RAY),  // +100
      mint(200, 0, ALICE, 5, 15, RAY),   // phantom: -(15-5) = -10
      mint(300, 0, ALICE, 7, 17, RAY),   // phantom: -(17-7) = -10
    ];
    let result = process_user_token_events(&events, ALICE, TOKEN, RAY, false).unwrap();
    assert_eq!(result.scaled_balance, 80);
  }

  /// A phantom-mint that completely zeroes out a user's position — the limiting case
  /// where the user repays exactly enough that the scaled balance lands at 0. Verifies
  /// the running scaled_balance accumulator goes to zero (not negative, not clamped).
  #[test]
  fn phantom_mint_can_drain_position_to_zero() {
    // setup +100 scaled, then phantom debits exactly 100 ((bi-value)/index = 100/1.0).
    let events = vec![
      mint(100, 0, ALICE, 100, 0, RAY),
      mint(200, 0, ALICE, 50, 150, RAY),
    ];
    let result = process_user_token_events(&events, ALICE, TOKEN, RAY, false).unwrap();
    assert_eq!(result.scaled_balance, 0);
  }
}
