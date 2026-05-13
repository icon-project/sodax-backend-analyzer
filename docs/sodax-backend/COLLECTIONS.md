# SODAX Backend — MongoDB Collection Reference

All collections the backend writes to MongoDB. Field names are the **actual MongoDB keys** (camelCase, matching serde deserialization).

**Type conventions:**
- `Decimal128` in MongoDB → deserialize as `bson::Decimal128`, convert to `u128` or `U256` in Rust
- `Address` = 42-char hex string (`0x...`), stored lowercase
- `Hash` = 66-char hex string (`0x...`)
- All event collections share unique index `{ chainId, txHash, logIndex }`

---

## Raw Event Collections

### `intent_events`
**Writer:** data-aggregator (intents) | **Discriminator key:** `eventType`

| Field | Type | Notes |
|-------|------|-------|
| `_id` | ObjectId | |
| `txHash` | String | lowercase, indexed |
| `logIndex` | Number | |
| `chainId` | Number | |
| `blockNumber` | Number | |
| `intentHash` | String (Hash) | |
| `eventType` | String | discriminator — see below |

**Unique index:** `{ chainId: 1, txHash: 1, logIndex: 1 }`

**Discriminator variants:**

#### `eventType = "intent-created"`
| Field | Type | Notes |
|-------|------|-------|
| `intent.intentId` | String | NOT unique across intents |
| `intent.creator` | String (Address) | |
| `intent.inputToken` | String (Address) | |
| `intent.outputToken` | String (Address) | |
| `intent.inputAmount` | Decimal128 | |
| `intent.minOutputAmount` | Decimal128 | |
| `intent.deadline` | Decimal128 | |
| `intent.allowPartialFill` | Boolean | |
| `intent.srcChain` | Number | |
| `intent.dstChain` | Number | |
| `intent.srcAddress` | String (Address) | |
| `intent.dstAddress` | String (Address) | |
| `intent.solver` | String (Address) | |
| `intent.data` | String (Hash) | |

#### `eventType = "intent-filled"`
| Field | Type | Notes |
|-------|------|-------|
| `intentState.exists` | Boolean | |
| `intentState.remainingInput` | Decimal128 | |
| `intentState.receivedOutput` | Decimal128 | |
| `intentState.pendingPayment` | Boolean | |

#### `eventType = "intent-cancelled"`
No additional fields.

#### `eventType = "external-fill-failed"`
| Field | Type | Notes |
|-------|------|-------|
| `fillId` | Decimal128 | |
| `inputAmount` | Decimal128 | |
| `outputAmount` | Decimal128 | |
| `to` | String (Address) | |
| `token` | String (Address) | |

---

### `money_market_events`
**Writer:** data-aggregator (money-market) | **Discriminator key:** `eventType`

| Field | Type | Notes |
|-------|------|-------|
| `_id` | ObjectId | |
| `txHash` | String | lowercase, indexed |
| `logIndex` | Number | |
| `chainId` | Number | |
| `blockNumber` | Number | |
| `eventType` | String | discriminator — 17 variants |

**Unique index:** `{ chainId: 1, txHash: 1, logIndex: 1 }`

**Key discriminator variants:**

#### `eventType = "supply"` / `"borrow"` / `"repay"` / `"withdraw"`
| Field | Type | Notes |
|-------|------|-------|
| `reserve` | String (Address) | |
| `user` | String (Address) | |
| `onBehalfOf` or `repayer` or `to` | String (Address) | varies by type |
| `amount` | Decimal128 | |
| `referralCode` | Number | supply/borrow only |
| `interestRateMode` | Number | borrow only |
| `borrowRate` | Decimal128 | borrow only |
| `useATokens` | Boolean | repay only |

#### `eventType = "a-token-mint"` / `"a-token-burn"` / `"debt-token-mint"` / `"debt-token-burn"`
| Field | Type | Notes |
|-------|------|-------|
| `tokenAddress` | String (Address) | |
| `caller` or `from` | String (Address) | mint=caller, burn=from |
| `onBehalfOf` or `target` | String (Address) | mint=onBehalfOf, burn=target |
| `value` | Decimal128 | |
| `balanceIncrease` | Decimal128 | |
| `index` | Decimal128 | RAY-scaled (10^27) |

#### `eventType = "a-token-transfer"` / `"a-token-balance-transfer"`
| Field | Type | Notes |
|-------|------|-------|
| `tokenAddress` | String (Address) | |
| `from` | String (Address) | |
| `to` | String (Address) | |
| `value` | Decimal128 | |
| `index` | Decimal128 | balance-transfer only |

#### `eventType = "reserve-data-updated"`
| Field | Type | Notes |
|-------|------|-------|
| `reserve` | String (Address) | |
| `liquidityRate` | Decimal128 | |
| `stableBorrowRate` | Decimal128 | |
| `variableBorrowRate` | Decimal128 | |
| `liquidityIndex` | Decimal128 | RAY-scaled |
| `variableBorrowIndex` | Decimal128 | RAY-scaled |

---

### `amm_events`
**Writer:** data-aggregator (amm) | **Discriminator key:** `eventType`

| Field | Type | Notes |
|-------|------|-------|
| `_id` | ObjectId | |
| `txHash` | String | lowercase, indexed |
| `logIndex` | Number | |
| `chainId` | Number | |
| `blockNumber` | Number | |
| `eventType` | String | discriminator |

**Unique index:** `{ chainId: 1, txHash: 1, logIndex: 1 }`

#### `eventType = "swap"`
| Field | Type |
|-------|------|
| `id` | String (pool ID) |
| `sender` | String (Address) |
| `amount0` | Decimal128 |
| `amount1` | Decimal128 |
| `sqrtPriceX96` | Decimal128 |
| `liquidity` | Decimal128 |
| `tick` | Number |
| `fee` | Number |
| `protocolFee` | Number |

#### `eventType = "initialize"`
| Field | Type |
|-------|------|
| `id` | String (pool ID) |
| `currency0` | String (Address) |
| `currency1` | String (Address) |

#### `eventType = "transfer"`
| Field | Type |
|-------|------|
| `from` | String (Address) |
| `to` | String (Address) |
| `id` | Decimal128 (token ID) |

#### `eventType = "mint-position"`
| Field | Type |
|-------|------|
| `tokenId` | Decimal128 |

---

### `wallet_factory_events`
**Writer:** data-aggregator (wallet-factory) | **Discriminator key:** `eventType`

**Unique index:** `{ chainId: 1, txHash: 1, logIndex: 1 }`

#### `eventType = "deployed"`
| Field | Type |
|-------|------|
| `spokeAddress` | String |
| `deployedAddress` | String (Address) |

---

### `protocol_revenue_events`
**Writer:** data-aggregator (protocol-revenue) | **Discriminator key:** `eventType`

| Field | Type | Notes |
|-------|------|-------|
| `txHash` | String | lowercase, indexed |
| `logIndex` | Number | |
| `chainId` | Number | |
| `blockNumber` | Number | |
| `contractAddress` | String (Address) | lowercase |
| `tokenAddress` | String (Address) | lowercase |
| `amount` | Decimal128 | |
| `eventType` | String | discriminator |

**Indexes:**
- `{ chainId: 1, txHash: 1, logIndex: 1 }` (unique)
- `{ tokenAddress: 1, blockNumber: 1 }`
- `{ contractAddress: 1, blockNumber: 1 }`

#### `eventType = "minted-to-treasury"`
No additional fields.

#### `eventType = "fees-distributed-to-treasury"`
| Field | Type |
|-------|------|
| `treasuryAddress` | String (Address) |

#### `eventType = "fee-collected"`
No additional fields.

---

### `aggregator_events_metadata`
**Writer:** data-aggregator

| Field | Type | Notes |
|-------|------|-------|
| `address` | String | lowercase, contract address |
| `chainId` | Number | |
| `lastBlockNumber` | Decimal128 | last synced block |
| `aggregator` | String | LogAggregator enum value |

**Unique index:** `{ address: 1, chainId: 1, aggregator: 1 }`

---

## Transformed Collections

### `intent_journal`
**Writer:** data-transformator

| Field | Type | Notes |
|-------|------|-------|
| `intentHash` | String (Hash) | |
| `txHash` | String (Hash) | creation tx |
| `logIndex` | Number | |
| `chainId` | Number | |
| `blockNumber` | Number | |
| `open` | Boolean | default: true |
| `intent` | Object | same shape as intent-created's `intent` |
| `events` | Array | event history |
| `events[].eventType` | String | IntentEventType value |
| `events[].txHash` | String | |
| `events[].logIndex` | Number | |
| `events[].blockNumber` | Number | |
| `events[].intentState` | Object | optional, for filled events |
| `createdAt` | Date | |
| `updatedAt` | Date | |

**Indexes:**
- `{ intentHash: 1, open: 1 }` (unique, partial: `{ open: true }`)
- `{ intentHash: 1, txHash: 1, logIndex: 1 }` (unique)

---

### `orderbook`
**Writer:** data-transformator

| Field | Type | Notes |
|-------|------|-------|
| `intentState.exists` | Boolean | |
| `intentState.remainingInput` | Decimal128 | |
| `intentState.receivedOutput` | Decimal128 | |
| `intentState.pendingPayment` | Boolean | |
| `intentData.intentId` | String | NOT unique |
| `intentData.creator` | String (Address) | |
| `intentData.inputToken` | String (Address) | |
| `intentData.outputToken` | String (Address) | |
| `intentData.inputAmount` | Decimal128 | |
| `intentData.minOutputAmount` | Decimal128 | |
| `intentData.deadline` | Decimal128 | |
| `intentData.allowPartialFill` | Boolean | |
| `intentData.srcChain` | Number | |
| `intentData.dstChain` | Number | |
| `intentData.srcAddress` | String (Address) | |
| `intentData.dstAddress` | String (Address) | |
| `intentData.solver` | String (Address) | |
| `intentData.data` | String (Hash) | |
| `intentData.intentHash` | String (Hash) | |
| `intentData.txHash` | String (Hash) | |
| `intentData.blockNumber` | Number | |
| `createdAt` | Date | |
| `updatedAt` | Date | |

**Unique index:** `{ 'intentData.intentHash': 1 }`

---

### `reserve_tokens`
**Writer:** data-transformator

| Field | Type | Notes |
|-------|------|-------|
| `reserveAddress` | String (Address) | unique, lowercase |
| `symbol` | String | optional |
| `totalATokenBalance` | Decimal128 | default: 0 |
| `totalVariableDebtTokenBalance` | Decimal128 | default: 0 |
| `suppliers` | Array of String | addresses |
| `borrowers` | Array of String | addresses |
| `aTokenAddress` | String (Address) | unique, lowercase |
| `variableDebtTokenAddress` | String (Address) | unique, lowercase |
| `liquidityRate` | Decimal128 | |
| `stableBorrowRate` | Decimal128 | |
| `variableBorrowRate` | Decimal128 | |
| `liquidityIndex` | Decimal128 | RAY-scaled (10^27) |
| `variableBorrowIndex` | Decimal128 | RAY-scaled (10^27) |
| `blockNumber` | Number | last update block |
| `indexBlockNumber` | Number | last index update block |
| `createdAt` | Date | |
| `updatedAt` | Date | |

---

### `user_positions`
**Writer:** data-transformator

| Field | Type | Notes |
|-------|------|-------|
| `userAddress` | String (Address) | unique index |
| `positions` | Array | per-reserve positions |
| `positions[].reserveAddress` | String (Address) | |
| `positions[].aTokenAddress` | String (Address) | |
| `positions[].variableDebtTokenAddress` | String (Address) | |
| `positions[].blockNumber` | Number | |
| `positions[].aTokenBalance` | Decimal128 | scaled balance |
| `positions[].variableDebtTokenBalance` | Decimal128 | scaled balance |
| `positions[].aTokenBalanceHistory` | Array | `{ final, delta, eventId }` |
| `positions[].debtTokenBalanceHistory` | Array | `{ final, delta, eventId }` |
| `createdAt` | Date | |
| `updatedAt` | Date | |

**Unique index:** `{ userAddress: 1 }`

---

### `solver_volume`
**Writer:** data-transformator

| Field | Type | Notes |
|-------|------|-------|
| `intentHash` | String (Hash) | |
| `solver` | String (Address) | |
| `inputToken` | String (Address) | |
| `outputToken` | String (Address) | |
| `amount` | Decimal128 | |
| `chainId` | Number | |
| `blockNumber` | Number | |
| `txHash` | String (Hash) | |
| `logIndex` | Number | |
| `timestamp` | Date | |
| `data` | String (Hash) | |
| `createdAt` | Date | |
| `updatedAt` | Date | |

**Unique index:** `{ chainId: 1, txHash: 1, logIndex: 1, blockNumber: 1 }`

---

### `amm_candles`
**Writer:** data-transformator

| Field | Type | Notes |
|-------|------|-------|
| `poolId` | String | |
| `chainId` | Number | |
| `interval` | String | `1m`, `5m`, `15m`, `1h`, `4h`, `1d` |
| `timestamp` | Number | unix epoch |
| `open` | String | price as string |
| `high` | String | |
| `low` | String | |
| `close` | String | |
| `volume0` | String | default: "0" |
| `volume1` | String | default: "0" |
| `tradeCount` | Number | default: 0 |
| `createdAt` | Date | |
| `updatedAt` | Date | |

**Indexes:**
- `{ chainId: 1, poolId: 1, interval: 1, timestamp: 1 }` (unique)
- `{ chainId: 1, poolId: 1, interval: 1 }`

---

### `amm_tokens`
**Writer:** data-transformator

| Field | Type | Notes |
|-------|------|-------|
| `chainId` | Number | |
| `tokenAddress` | String (Address) | |
| `decimals` | Number | nullable |
| `symbol` | String | nullable |
| `name` | String | nullable |
| `poolIds` | Array of String | default: [] |
| `createdAt` | Date | |
| `updatedAt` | Date | |

**Indexes:**
- `{ chainId: 1, tokenAddress: 1 }` (unique)
- `{ chainId: 1, poolIds: 1 }`

---

### `amm_nft_positions`
**Writer:** data-transformator

| Field | Type | Notes |
|-------|------|-------|
| `owner` | String (Address) | |
| `currency0` | String (Address) | nullable |
| `currency1` | String (Address) | nullable |
| `poolId200` | String | nullable |
| `tokenId` | Decimal128 | |
| `createdAt` | Date | |
| `updatedAt` | Date | |

**Indexes:**
- `{ tokenId: 1 }` (unique)
- `{ owner: 1, tokenId: 1 }`

---

### `partner_asset`
**Writer:** data-transformator

| Field | Type | Notes |
|-------|------|-------|
| `receiver` | String (Address) | |
| `asset` | String (Address) | |
| `chainId` | Number | |
| `lastBlockNumber` | Number | |
| `outputs` | Map | keyed by output token address |
| `outputs.<outputToken>.totalVolumeOut` | Decimal128 | |
| `outputs.<outputToken>.totalFeeIn` | Decimal128 | |
| `outputs.<outputToken>.txCount` | Number | |
| `createdAt` | Date | |
| `updatedAt` | Date | |

**Unique index:** `{ receiver: 1, asset: 1, chainId: 1 }`

---

### `sodax_supply`
**Writer:** data-aggregator (sodax-supply)

| Field | Type | Notes |
|-------|------|-------|
| `_id` | String | fixed: `"singleton"` |
| `block` | String | |
| `totalSupply` | String | |
| `lockedSupply` | String | |
| `circulatingSupply` | String | |
| `daoFund` | String | |
| `icxMigration` | String | |
| `balnMigration` | String | |
| `createdAt` | Date | |
| `updatedAt` | Date | |

---

## Metadata Collections (Transformator)

All share the singleton pattern (`_id = "singleton"`) and the same shape:

| Collection | Writer |
|------------|--------|
| `intent_journal_metadata` | data-transformator |
| `money_market_events_metadata` | data-transformator |
| `amm_events_metadata` | data-transformator |
| `orderbook_metadata` | data-transformator |
| `solver_volume_metadata` | data-transformator |
| `protocol_revenue_events_metadata` | data-transformator |

**Fields:**

| Field | Type | Notes |
|-------|------|-------|
| `_id` | String | `"singleton"` |
| `lastProcessedBlock` | Number | |
| `processedEventKeys` | Array of String | `"txHash-logIndex"` format |
| `createdAt` | Date | |
| `updatedAt` | Date | |

---

## Stateful Collections

### `stateful_users`
**Writer:** stateful-api

| Field | Type | Notes |
|-------|------|-------|
| `address` | String | trimmed |
| `chain` | String | Chains enum value |
| `signature` | String | indexed |
| `message` | String | |
| `createdAt` | Date | |
| `updatedAt` | Date | |

**Unique index:** `{ signature: 1, address: 1, chain: 1 }`

---

### `stateful_submit_swap_tx`
**Writer:** stateful-api (creates), task-executor (updates status)

| Field | Type | Notes |
|-------|------|-------|
| `txHash` | String | indexed |
| `srcChainId` | String | SpokeChainId |
| `walletAddress` | String | indexed |
| `intent` | Object | same Intent shape |
| `relayData` | String | hex-encoded |
| `status` | String | SubmitSwapTxStatus, default: `"pending"` |
| `failedAtStep` | String | optional |
| `failureReason` | String | optional |
| `failedAttempts` | Number | default: 0 |
| `lastFailedAt` | Date | optional |
| `result.dstIntentTxHash` | String | optional |
| `result.packetData` | Object | optional |
| `result.intent_hash` | String | optional, indexed |
| `createdAt` | Date | |
| `updatedAt` | Date | |

**Indexes:**
- `{ txHash: 1, srcChainId: 1 }` (unique)
- `{ status: 1, failedAttempts: 1, lastFailedAt: 1, createdAt: 1 }`

---

### `stateful_partner_naming`
**Writer:** stateful-api

| Field | Type | Notes |
|-------|------|-------|
| `walletAddress` | String | unique, lowercase, trimmed |
| `name` | String | nullable, unique (sparse) |
| `createdAt` | Date | |
| `updatedAt` | Date | |

---

### `stateful_signature_attestations`
**Writer:** stateful-api

| Field | Type | Notes |
|-------|------|-------|
| `address` | String | trimmed, lowercase |
| `purpose` | String | trimmed |
| `message` | String | |
| `signature` | String | |
| `createdAt` | Date | |
| `updatedAt` | Date | |

**Indexes:**
- `{ address: 1, purpose: 1 }` (unique)
- `{ createdAt: -1 }`

---

### `stateful_signature_challenges`
**Writer:** stateful-api

| Field | Type | Notes |
|-------|------|-------|
| `walletAddress` | String | unique, lowercase, trimmed |
| `challengesByPurpose` | Map | purpose → `{ nonce, generatedAt }` |
| `createdAt` | Date | |
| `updatedAt` | Date | |

---

## Other Collections

### `portfolio_snapshots`
**Writer:** task-executor

| Field | Type | Notes |
|-------|------|-------|
| `address` | String (Address) | |
| `chainId` | Number | |
| `blockNumber` | Number | |
| `blockTimestamp` | Date | |
| `tokenBalances` | Array | `{ tokenAddress, symbol, balance (Decimal128) }` |
| `moneyMarketPositions` | Array | `{ reserveAddress, aTokenBalance, variableDebtTokenBalance }` |
| `lpPositions` | Array | `{ tokenId, currency0, currency1, tickLower, tickUpper, liquidity }` |
| `createdAt` | Date | |
| `updatedAt` | Date | |

**Indexes:**
- `{ address: 1, chainId: 1, blockNumber: 1 }` (unique)
- `{ address: 1, chainId: 1, blockTimestamp: 1 }`
