# SODAX Backend — Enum Reference

All enum/const values used across the backend. Use these for filtering queries and validating data.

---

## Event Type Enums

### IntentEventType
```
intent-created
intent-cancelled
intent-filled
external-fill-failed
```

### MoneyMarketEventType
```
borrow
supply
repay
withdraw
user-emode-set
flash-loan
isolation-mode-total-debt-updated
liquidation-call
mint-unbacked
back-unbacked
a-token-transfer
a-token-balance-transfer
a-token-burn
a-token-mint
debt-token-burn
debt-token-mint
reserve-data-updated
```

### AmmEventType
```
transfer
mint-position
swap
initialize
```

### WalletFactoryEventType
```
deployed
```

### ProtocolRevenueEventType
```
minted-to-treasury
fees-distributed-to-treasury
fee-collected
```

---

## Aggregator Enums

### LogAggregator
```
intents
money-market
wallet-factory
amm
protocol-revenue
```
> Do not change existing values — only append.

### DataAggregator
```
sodax-supply
```

Combined as `Aggregator = { ...LogAggregator, ...DataAggregator }`.

---

## Chain Enums

### ChainId
| Name | Value |
|------|-------|
| SONIC | `146` |
| SONIC_TESTNET | `64165` |

### ChainType
```
evm
```

### Chains (multi-chain signature support)
```
EVM
STELLAR
SUI
SOLANA
INJECTIVE
```

---

## AMM Enums

### AmmInterval
```
1m
5m
15m
1h
4h
1d
```

---

## Stateful Enums

### SubmitSwapTxStatus (type union, not enum)
```
pending
verifying
verified
relaying
relayed
posting_execution
executed
failed
```

Status flow: `pending → verifying → verified → relaying → relayed → posting_execution → executed`
Any step can transition to `failed`.

---

## Incident Manager Enums

### IncidentFlowTypes
```
MONEY_MARKET_CORRUPTED
AMM_CORRUPTED
INTENT_CREATED_EVENT_NOT_FOUND
```

### IncidentCodeTypes
```
INVARIANT_BROKEN
DERIVED_MISMATCH
SCHEMA_DRIFT
```

### IncidentStatuses
```
pending
running
resolved
failed
```

### TargetStatuses
```
pending
running
done
failed
```

---

## Other Enums

### EnvType
```
dev
prod
test
```

### LogLevel
```
debug
info
warn
error
```

---

## CollectionNames
Complete mapping of enum key → MongoDB collection name:

| Enum Key | Collection Name |
|----------|----------------|
| AGGREGATOR_EVENTS_METADATA | `aggregator_events_metadata` |
| INTENT_EVENTS | `intent_events` |
| WALLET_FACTORY_EVENTS | `wallet_factory_events` |
| MONEY_MARKET_EVENTS | `money_market_events` |
| AMM_EVENTS | `amm_events` |
| INTENT_JOURNAL_METADATA | `intent_journal_metadata` |
| INTENT_JOURNAL | `intent_journal` |
| MONEY_MARKET_EVENTS_METADATA | `money_market_events_metadata` |
| RESERVE_TOKENS | `reserve_tokens` |
| USER_POSITIONS | `user_positions` |
| AMM_EVENTS_METADATA | `amm_events_metadata` |
| AMM_NFT_POSITIONS | `amm_nft_positions` |
| AMM_CANDLES | `amm_candles` |
| AMM_TOKENS | `amm_tokens` |
| ORDERBOOK_METADATA | `orderbook_metadata` |
| ORDERBOOK | `orderbook` |
| SOLVER_VOLUME_METADATA | `solver_volume_metadata` |
| SOLVER_VOLUME | `solver_volume` |
| PARTNER_ASSET | `partner_asset` |
| PROTOCOL_REVENUE_EVENTS | `protocol_revenue_events` |
| PROTOCOL_REVENUE_EVENTS_METADATA | `protocol_revenue_events_metadata` |
| SODAX_SUPPLY | `sodax_supply` |
| STATEFUL_USERS | `stateful_users` |
| STATEFUL_SUBMIT_SWAP_TX | `stateful_submit_swap_tx` |
| PARTNERS | `stateful_partner_naming` |
| SIGNATURE_ATTESTATIONS | `stateful_signature_attestations` |
| SIGNATURE_CHALLENGES | `stateful_signature_challenges` |
| PORTFOLIO_SNAPSHOTS | `portfolio_snapshots` |
