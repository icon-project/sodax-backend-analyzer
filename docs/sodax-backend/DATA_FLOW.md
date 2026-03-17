# SODAX Backend — Data Flow & Collection Ownership

## Architecture

```
Smart Contracts (Sonic chain, chainId 146/64165)
         │
         ▼
┌─────────────────┐     ┌──────────────┐
│ Data Aggregator  │────▶│   MongoDB    │◀──── Stateful API (user/partner data)
│ (event listener) │     │              │◀──── Task Executor (portfolio snapshots)
└─────────────────┘     └──────┬───────┘
                               │
                               ▼
                  ┌─────────────────────┐
                  │ Data Transformator   │──▶ MongoDB (transformed collections)
                  │ (event processing)   │
                  └─────────────────────┘
                               │
                               ▼
                        ┌─────────┐
                        │   API   │──▶ Redis cache ──▶ Frontend
                        └─────────┘
```

## Write Ownership Table

**Rule:** Each collection has exactly ONE writer service. Reads are unrestricted.

| Collection | Writer | Notes |
|------------|--------|-------|
| **Raw Events** | | |
| `intent_events` | data-aggregator | intents log aggregator |
| `money_market_events` | data-aggregator | money-market log aggregator |
| `amm_events` | data-aggregator | amm log aggregator |
| `wallet_factory_events` | data-aggregator | wallet-factory log aggregator |
| `protocol_revenue_events` | data-aggregator | protocol-revenue log aggregator |
| `aggregator_events_metadata` | data-aggregator | tracks sync progress per contract |
| `sodax_supply` | data-aggregator | sodax-supply data aggregator (singleton) |
| **Transformed** | | |
| `intent_journal` | data-transformator | |
| `intent_journal_metadata` | data-transformator | singleton |
| `orderbook` | data-transformator | |
| `orderbook_metadata` | data-transformator | singleton |
| `reserve_tokens` | data-transformator | |
| `user_positions` | data-transformator | |
| `money_market_events_metadata` | data-transformator | singleton |
| `amm_candles` | data-transformator | |
| `amm_tokens` | data-transformator | |
| `amm_nft_positions` | data-transformator | |
| `amm_events_metadata` | data-transformator | singleton |
| `solver_volume` | data-transformator | |
| `solver_volume_metadata` | data-transformator | singleton |
| `partner_asset` | data-transformator | |
| `protocol_revenue_events_metadata` | data-transformator | singleton |
| **Stateful** | | |
| `stateful_users` | stateful-api | |
| `stateful_partner_naming` | stateful-api | |
| `stateful_signature_attestations` | stateful-api | |
| `stateful_signature_challenges` | stateful-api | |
| `stateful_submit_swap_tx` | stateful-api + task-executor | **exception**: stateful-api creates, task-executor updates status |
| **Other** | | |
| `portfolio_snapshots` | task-executor | scheduled snapshots |

## Key Domain Rules

### Intent Identity
- `intentId` is **NOT unique** — the same intentId can appear multiple times (e.g., after cancellation and re-creation).
- Always use `intentHash` as the unique identifier for an intent.
- The `intentHash` is derived from the intent data and is guaranteed unique.

### Decimal128 / BigInt Handling
- All blockchain quantities (amounts, balances, indexes) are stored as `Decimal128` in MongoDB.
- The backend converts `BigInt ↔ Decimal128` via getters/setters on Mongoose schemas.
- In Rust: deserialize `Decimal128` from BSON, then convert to `u128` or `U256`.
- String fields like `sodax_supply.totalSupply` and `amm_candles.open/high/low/close` store numeric values as plain strings (not Decimal128).

### Aave Scaled Balance Math
- All aToken and debtToken balances in `user_positions` are **scaled balances** (not real balances).
- To get real balance: `realBalance = scaledBalance × currentIndex / RAY`
- `RAY = 10^27` (Aave's precision constant).
- Current indexes are in `reserve_tokens.liquidityIndex` and `reserve_tokens.variableBorrowIndex`.

### Event Discriminator Pattern
Four event collections use Mongoose discriminators on the `eventType` field:
- `intent_events` — 4 variants
- `money_market_events` — 17 variants (11 with schemas, 6 aggregated but not deserialized by transformator)
- `amm_events` — 4 variants
- `protocol_revenue_events` — 3 variants

Each variant adds different fields to the base event document. When querying, filter by `eventType` to know which fields are present.

### Metadata / Singleton Pattern
All `*_metadata` collections (except `aggregator_events_metadata`) use a single document with `_id: "singleton"`. They track the last processed block and a list of processed event keys (`"txHash-logIndex"` format) to enable idempotent reprocessing.

### Collection Name Convention
All MongoDB collection names use **snake_case** (e.g., `intent_events`, `user_positions`). This matches the `CollectionNames` enum in the backend.
