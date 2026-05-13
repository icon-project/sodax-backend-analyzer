// Ports `extractFeeIntentData` + `tryDecodeIntentData` from
// sodax-backend `packages/shared-utils/src/utils/common-utils.ts:36-138, 419-438`.
//
// Encoded intent data uses a custom 1-byte dataType prefix followed by an
// ABI-encoded payload:
//
//   0x{dataType:2 hex}{abi-encoded payload}
//
// dataType values (IntentDataType enum in the JS source):
//   0 = ARRAY: a dynamic array of (uint8 dataType, bytes data) tuples
//   1 = FEE:   (uint256 fee, address receiver)
//   2 = HOOK:  ignored for partner_asset recomputation
//
// We DFS into ARRAY entries and return the first FEE encountered, matching
// the JS `extractFeeIntentData` semantics exactly.

use alloy::primitives::{Address, U256};
use alloy::sol_types::SolValue;

alloy::sol! {
  struct ArrayEntry {
    uint8 dataType;
    bytes data;
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeeIntentData {
  pub fee: U256,
  pub receiver: Address,
}

pub fn extract_fee_from_intent_data(hex_str: &str) -> Option<FeeIntentData> {
  let bytes = decode_hex(hex_str)?;
  extract_fee_from_bytes(&bytes)
}

fn extract_fee_from_bytes(bytes: &[u8]) -> Option<FeeIntentData> {
  if bytes.is_empty() {
    return None;
  }
  let data_type = bytes[0];
  let payload = &bytes[1..];

  match data_type {
    1 => decode_fee(payload),
    0 => decode_array_and_find_fee(payload),
    _ => None,
  }
}

fn decode_fee(payload: &[u8]) -> Option<FeeIntentData> {
  let (fee, receiver) = <(U256, Address) as SolValue>::abi_decode_params(payload).ok()?;
  Some(FeeIntentData { fee, receiver })
}

fn decode_array_and_find_fee(payload: &[u8]) -> Option<FeeIntentData> {
  // ARRAY payload is encoded as a single dynamic array of (uint8, bytes) tuples.
  let entries: Vec<ArrayEntry> = <Vec<ArrayEntry> as SolValue>::abi_decode_params(payload).ok()?;
  for entry in entries {
    let mut nested = Vec::with_capacity(1 + entry.data.len());
    nested.push(entry.dataType);
    nested.extend_from_slice(entry.data.as_ref());
    if let Some(result) = extract_fee_from_bytes(&nested) {
      return Some(result);
    }
  }
  None
}

fn decode_hex(hex_str: &str) -> Option<Vec<u8>> {
  let trimmed = hex_str.strip_prefix("0x").unwrap_or(hex_str);
  if trimmed.is_empty() {
    return None;
  }
  alloy::hex::decode(trimmed).ok()
}

#[cfg(test)]
mod tests {
  use super::*;
  use alloy::primitives::{address, Bytes};

  fn encode_fee_blob(fee: U256, receiver: Address) -> Vec<u8> {
    let payload = (fee, receiver).abi_encode_params();
    let mut blob = Vec::with_capacity(1 + payload.len());
    blob.push(1u8);
    blob.extend_from_slice(&payload);
    blob
  }

  fn encode_array_blob(entries: Vec<(u8, Vec<u8>)>) -> Vec<u8> {
    let entries_alloy: Vec<ArrayEntry> = entries
      .into_iter()
      .map(|(t, d)| ArrayEntry {
        dataType: t,
        data: Bytes::from(d),
      })
      .collect();
    let payload = entries_alloy.abi_encode_params();
    let mut blob = Vec::with_capacity(1 + payload.len());
    blob.push(0u8);
    blob.extend_from_slice(&payload);
    blob
  }

  fn to_hex(bytes: &[u8]) -> String {
    format!("0x{}", alloy::hex::encode(bytes))
  }

  #[test]
  fn decodes_fee() {
    let fee = U256::from(123_456_789u64);
    let receiver = address!("0x0Ab764AB3816cD036Ea951bE973098510D8105A6");
    let hex = to_hex(&encode_fee_blob(fee, receiver));

    let got = extract_fee_from_intent_data(&hex).unwrap();
    assert_eq!(got.fee, fee);
    assert_eq!(got.receiver, receiver);
  }

  #[test]
  fn returns_none_for_hook() {
    let payload = (
      address!("0x0000000000000000000000000000000000000001"),
      Bytes::from(vec![0xde, 0xad]),
    )
      .abi_encode_params();
    let mut blob = vec![2u8];
    blob.extend_from_slice(&payload);
    let hex = to_hex(&blob);

    assert!(extract_fee_from_intent_data(&hex).is_none());
  }

  #[test]
  fn returns_none_for_unknown_data_type() {
    let blob = vec![0x99u8, 0x00, 0x00];
    let hex = to_hex(&blob);

    assert!(extract_fee_from_intent_data(&hex).is_none());
  }

  #[test]
  fn returns_none_for_empty_and_invalid() {
    assert!(extract_fee_from_intent_data("").is_none());
    assert!(extract_fee_from_intent_data("0x").is_none());
    assert!(extract_fee_from_intent_data("0xnothex").is_none());
    assert!(extract_fee_from_intent_data("0x01").is_none()); // FEE prefix with no payload
  }

  #[test]
  fn decodes_array_with_single_fee_entry() {
    let fee = U256::from(7777u64);
    let receiver = address!("0x70178089842Be7f8E4726b33F0D1569dB8021fAa");
    let fee_blob_without_prefix = (fee, receiver).abi_encode_params();
    let entries = vec![(1u8, fee_blob_without_prefix)];
    let hex = to_hex(&encode_array_blob(entries));

    let got = extract_fee_from_intent_data(&hex).unwrap();
    assert_eq!(got.fee, fee);
    assert_eq!(got.receiver, receiver);
  }

  #[test]
  fn dfs_finds_fee_after_hook_entry() {
    let fee = U256::from(42u64);
    let receiver = address!("0xdB7BdA65c3a1C51D64dC4444e418684677334109");

    let hook_payload = (
      address!("0x0000000000000000000000000000000000000abc"),
      Bytes::from(vec![0x01, 0x02]),
    )
      .abi_encode_params();
    let fee_payload = (fee, receiver).abi_encode_params();

    let entries = vec![(2u8, hook_payload), (1u8, fee_payload)];
    let hex = to_hex(&encode_array_blob(entries));

    let got = extract_fee_from_intent_data(&hex).unwrap();
    assert_eq!(got.fee, fee);
    assert_eq!(got.receiver, receiver);
  }

  #[test]
  fn decodes_nested_array_containing_fee() {
    let fee = U256::from(1u128 << 100);
    let receiver = address!("0x0c09e69a4528945de6d16c7E469deA6996Fdf636");
    let inner_fee_payload = (fee, receiver).abi_encode_params();
    let inner_entries = vec![(1u8, inner_fee_payload)];
    let inner_array_blob = encode_array_blob(inner_entries);
    // strip the outer prefix to use as inner `data`
    let inner_array_payload = inner_array_blob[1..].to_vec();

    let outer_entries = vec![(0u8, inner_array_payload)];
    let hex = to_hex(&encode_array_blob(outer_entries));

    let got = extract_fee_from_intent_data(&hex).unwrap();
    assert_eq!(got.fee, fee);
    assert_eq!(got.receiver, receiver);
  }

  #[test]
  fn returns_none_when_array_has_no_fee() {
    let hook_payload = (
      address!("0x000000000000000000000000000000000000000A"),
      Bytes::from(vec![]),
    )
      .abi_encode_params();
    let entries = vec![(2u8, hook_payload)];
    let hex = to_hex(&encode_array_blob(entries));

    assert!(extract_fee_from_intent_data(&hex).is_none());
  }
}
