# CLI Reference

Complete reference for every flag accepted by the analyzer CLI.

The binary is invoked as either `target/release/sodax_backend_analizer <flags...>` after `cargo build --release` (note: the package name in `Cargo.toml` is `sodax_backend_analizer` — with underscores and the legacy "analizer" spelling), or — most commonly during development — `cargo run -- <flags...>`. This document uses the latter form in examples; both behave identically.

If invoked with no flags, the tool prints the help message and exits with status 0 (equivalent to passing `--help`).

## Contents

- [Conventions](#conventions)
- [Quick reference](#quick-reference)
- [Meta & help](#meta--help)
- [Read-only data lookup](#read-only-data-lookup)
- [Token data](#token-data)
- [User position & balance](#user-position--balance)
- [Event queries](#event-queries)
- [Position inspection](#position-inspection)
- [Timestamp validation](#timestamp-validation)
- [Reserve index validation](#reserve-index-validation)
- [Single-target balance validation](#single-target-balance-validation)
- [Bulk balance validation](#bulk-balance-validation)
- [Event-replay validation](#event-replay-validation)
- [Partner asset validation](#partner-asset-validation)
- [Modifiers](#modifiers)
- [Combination rules](#combination-rules)
- [Reports](#reports)

## Conventions

Placeholders used below:

- `<USER_ADDRESS>` — an EOA / wallet address (`0x...`)
- `<TOKEN_ADDRESS>` — an ERC-20 / reserve / aToken / variable-debt-token address
- `<RESERVE_ADDRESS>` — the underlying reserve asset address (not the aToken or debt token)
- `<BLOCK_NUMBER>` — a `u64` block height
- `<PARTNER_ADDRESS>` — a partner-asset receiver address

Source of truth for the parser and dispatch:
- Flag enum: [`src/structs.rs`](../src/structs.rs)
- Parsing & validation rules: [`src/cli.rs`](../src/cli.rs)
- Dispatch: [`src/main.rs`](../src/main.rs)
- Behavior: [`src/handlers.rs`](../src/handlers.rs)
- Built-in help text: [`src/constants.rs`](../src/constants.rs)

## Quick reference

| Flag | Arg | Companion | Purpose |
|---|---|---|---|
| `--help` | — | none | Print help and exit |
| `--no-report` | — | any | Disable automatic report file generation |
| `--all-tokens` | — | none | List every reserve token in the DB |
| `--last-block` | — | none | Latest block from RPC |
| `--orderbook` | — | none | Dump pending orderbook intents |
| `--get-all-users` | — | none | Print all user addresses |
| `--get-all-reserves` | — | none | Print reserve addresses + symbols |
| `--get-all-a-token` | — | none | Print aToken addresses + symbols |
| `--get-all-debt-token` | — | none | Print variable-debt-token addresses + symbols |
| `--reserve-token` | address | — | Show reserve data; also acts as token selector |
| `--a-token` | address | — | Show reserve data; also acts as token selector |
| `--debt-token` | address | — | Show reserve data; also acts as token selector |
| `--user-position` | user | one token flag | Show user position for a token |
| `--balance-of` | user | one token flag, opt. `--block` | On-chain ERC-20 balance |
| `--block` | block | `--balance-of` | Query a specific block |
| `--get-token-events` | token | none | Events for a token |
| `--get-user-events` | user | none | Events for a user |
| `--inspect-user-position` | user | `--a-token` or `--debt-token` | Detect missed events vs. balance history |
| `--timestamp-coverage` | — | none | % of docs with non-null timestamps |
| `--validate-timestamps` | opt. count (1-100) | none | Validate DB timestamps vs. on-chain block timestamps |
| `--validate-reserve-indexes` | reserve | none | Validate liquidity/borrow indexes for one reserve |
| `--validate-all-reserve-indexes` | — | none | Validate indexes across all reserves |
| `--validate-user-supply` | user | `--reserve-token` (+ opt. `--scaled`) | Validate one user's aToken balance |
| `--validate-user-borrow` | user | `--reserve-token` (+ opt. `--scaled`) | Validate one user's debt balance |
| `--validate-token-supply` | — | `--reserve-token` (+ opt. `--scaled`) | Validate a reserve's total aToken supply |
| `--validate-token-borrow` | — | `--reserve-token` (+ opt. `--scaled`) | Validate a reserve's total debt supply |
| `--validate-user-all` | user | opt. `--scaled` | Validate every position for one user |
| `--validate-users-all` | — | opt. `--scaled` | Validate every position for every user |
| `--validate-token-all` | — | opt. `--scaled` | Validate every reserve |
| `--validate-all` | — | opt. `--scaled` | Validate every reserve + every user |
| `--calculate-from-events` | user | one token flag | Reconstruct balance from event history |
| `--calculate-from-events-reserve` | reserve | opt. `--a-token-only`/`--debt-token-only`, `--verbose`, `--json` | Reconstruct balances for every user in a reserve |
| `--calculate-from-events-reserve-all` | — | opt. `--a-token-only`/`--debt-token-only`, `--json` | Reconstruct balances for every user in every reserve (market-wide health check) |
| `--a-token-only` | — | `--calculate-from-events-reserve` / `--calculate-from-events-reserve-all` | Limit reserve replay to supply side |
| `--debt-token-only` | — | `--calculate-from-events-reserve` / `--calculate-from-events-reserve-all` | Limit reserve replay to debt side |
| `--verbose` | — | `--calculate-from-events-reserve` | Print full per-user replay block instead of the compact table (not valid with the -all variant) |
| `--validate-from-events` | user | opt. `--reserve-token` | 3-way validate (events vs DB vs chain) for one user |
| `--validate-from-events-all` | — | none | 3-way validate every user |
| `--validate-partner-asset` | — | opt. `--partner`, `--json`, `--threshold` | Recompute `partner_asset` aggregates and report drift |
| `--partner` | address | `--validate-partner-asset` | Restrict to one receiver |
| `--json` | — | `--validate-partner-asset`, `--calculate-from-events-reserve`, or `--calculate-from-events-reserve-all` | Emit JSON output |
| `--threshold` | float | `--validate-partner-asset` | Suppress rows within ±PCT of 1.0 (default 0.0001) |
| `--scaled` | — | validation flags | Compare scaled (raw) balances instead of real |

---

## Meta & help

### `--help`

Print the full help message and exit. Cannot be combined with any other flag.

```bash
cargo run -- --help
```

### `--no-report`

Disable automatic report file generation for the current invocation. By default every command (except `--help`) writes its output to `reports/report_<unix_timestamp>.txt`. This flag is consumed in `main.rs` before flag parsing, so it pairs with any other command.

```bash
cargo run -- --validate-all --no-report
```

## Read-only data lookup

These flags all run standalone — they cannot be combined with anything else.

### `--all-tokens`

Lists every entry in the `reserve_tokens` collection with full data per reserve.

```bash
cargo run -- --all-tokens
```

### `--last-block`

Queries the RPC provider for the latest block number.

```bash
cargo run -- --last-block
```

### `--orderbook`

Dumps the entire `orderbook` collection (pending intents).

```bash
cargo run -- --orderbook
```

### `--get-all-users`

Prints every user address known to the DB (from `user_positions`).

```bash
cargo run -- --get-all-users
```

### `--get-all-reserves`

Prints `(reserveAddress, symbol)` pairs for every reserve.

```bash
cargo run -- --get-all-reserves
```

### `--get-all-a-token`

Prints `(aTokenAddress, symbol)` pairs for every reserve.

```bash
cargo run -- --get-all-a-token
```

### `--get-all-debt-token`

Prints `(variableDebtTokenAddress, symbol)` pairs for every reserve.

```bash
cargo run -- --get-all-debt-token
```

## Token data

The three token flags below have a dual purpose:

1. **Standalone** — look up reserve data via that token type and print it.
2. **As a selector for another flag** (e.g. `--balance-of`, `--user-position`, `--calculate-from-events`, etc.) — pick which of the three addresses on a reserve to operate against.

They are mutually exclusive (`--reserve-token` + `--a-token` is rejected, etc.).

### `--reserve-token <RESERVE_ADDRESS>`

Reserve data keyed by the underlying asset address.

```bash
cargo run -- --reserve-token 0x1234...
```

### `--a-token <ATOKEN_ADDRESS>`

Reserve data keyed by the aToken address.

```bash
cargo run -- --a-token 0x5c50...
```

### `--debt-token <DEBT_TOKEN_ADDRESS>`

Reserve data keyed by the variable-debt-token address.

```bash
cargo run -- --debt-token 0x5c50...
```

## User position & balance

### `--user-position <USER_ADDRESS>`

Returns the user's position for the selected token. Requires exactly one of `--reserve-token`, `--a-token`, or `--debt-token`.

```bash
cargo run -- --user-position 0xuser... --reserve-token 0xtoken...
```

### `--balance-of <USER_ADDRESS>`

On-chain ERC-20 balance for the user, against the selected token. Requires exactly one of `--reserve-token`, `--a-token`, or `--debt-token`. Optionally takes `--block` to query at a specific height.

```bash
cargo run -- --balance-of 0xuser... --reserve-token 0xtoken...
cargo run -- --balance-of 0xuser... --a-token 0xatoken... --block 12345678
```

### `--block <BLOCK_NUMBER>`

Pin the block height for an on-chain query. Only valid alongside `--balance-of`.

```bash
cargo run -- --balance-of 0xuser... --reserve-token 0xtoken... --block 12345678
```

## Event queries

### `--get-token-events <TOKEN_ADDRESS>`

Print all events recorded for a token (works against any reserve / aToken / debt token address). Standalone — combines with nothing else.

```bash
cargo run -- --get-token-events 0xtoken...
```

### `--get-user-events <USER_ADDRESS>`

Print all events recorded for a user. Standalone — combines with nothing else.

```bash
cargo run -- --get-user-events 0xuser...
```

## Position inspection

### `--inspect-user-position <USER_ADDRESS>`

Audits a user's balance history for one token by cross-checking two sources:

1. Money market events from the `money_market_events` collection.
2. Event IDs recorded against the user's balance history in `user_positions` (plus the dedicated `user_balance_events` collection).

Reports any events that exist in (1) but are missing from (2) — i.e. events the indexer failed to attribute to the user's balance history.

Requires exactly one of `--a-token` or `--debt-token` (you must pick which token type to inspect).

```bash
cargo run -- --inspect-user-position 0xuser... --a-token 0xatoken...
cargo run -- --inspect-user-position 0xuser... --debt-token 0xdebt...
```

Output is JSON with `eventsOnMoneyMarketEventCollection`, `eventsOnUserBalanceHistory`, `eventsMissedCount`, and `missedEvents`.

## Timestamp validation

### `--timestamp-coverage`

Reports the percentage of documents (across event-bearing collections) that have a non-null `timestamp` field. Standalone.

```bash
cargo run -- --timestamp-coverage
```

### `--validate-timestamps [COUNT]`

Validates database timestamps against on-chain block timestamps. Optional integer argument `COUNT` (1-100) limits the number of entries checked; omitting it validates all entries.

```bash
cargo run -- --validate-timestamps
cargo run -- --validate-timestamps 50
```

## Reserve index validation

### `--validate-reserve-indexes <RESERVE_ADDRESS>`

For one reserve, validates the stored liquidity index and variable borrow index against the on-chain values returned by the pool. Standalone.

```bash
cargo run -- --validate-reserve-indexes 0xreserve...
```

### `--validate-all-reserve-indexes`

Runs `--validate-reserve-indexes` against every reserve sequentially. Standalone.

```bash
cargo run -- --validate-all-reserve-indexes
```

## Single-target balance validation

All four flags below require `--reserve-token` and optionally accept `--scaled` (see [Modifiers](#modifiers)). They compare the database value to the on-chain value and report `database_amount`, `on_chain_amount`, `difference`, and `percentage`.

### `--validate-user-supply <USER_ADDRESS>`

Validate one user's supplied (aToken) balance for a reserve.

```bash
cargo run -- --validate-user-supply 0xuser... --reserve-token 0xtoken...
cargo run -- --validate-user-supply 0xuser... --reserve-token 0xtoken... --scaled
```

### `--validate-user-borrow <USER_ADDRESS>`

Validate one user's variable-debt balance for a reserve.

```bash
cargo run -- --validate-user-borrow 0xuser... --reserve-token 0xtoken...
cargo run -- --validate-user-borrow 0xuser... --reserve-token 0xtoken... --scaled
```

### `--validate-token-supply`

Validate the total aToken supply for a reserve (sum across users vs. on-chain `totalSupply`).

```bash
cargo run -- --validate-token-supply --reserve-token 0xtoken...
cargo run -- --validate-token-supply --reserve-token 0xtoken... --scaled
```

### `--validate-token-borrow`

Validate the total variable-debt supply for a reserve.

```bash
cargo run -- --validate-token-borrow --reserve-token 0xtoken...
cargo run -- --validate-token-borrow --reserve-token 0xtoken... --scaled
```

## Bulk balance validation

These run the four single-target checks across many targets, with bounded concurrency (10 users × 5 positions per user). All optionally accept `--scaled`.

### `--validate-user-all <USER_ADDRESS>`

Validate every position belonging to one user.

```bash
cargo run -- --validate-user-all 0xuser...
cargo run -- --validate-user-all 0xuser... --scaled
```

### `--validate-users-all`

Validate every position for every user.

```bash
cargo run -- --validate-users-all
cargo run -- --validate-users-all --scaled
```

### `--validate-token-all`

Validate every reserve's totals (supply + borrow).

```bash
cargo run -- --validate-token-all
cargo run -- --validate-token-all --scaled
```

### `--validate-all`

Run `--validate-token-all` + `--validate-users-all` in one invocation.

```bash
cargo run -- --validate-all
cargo run -- --validate-all --scaled
```

## Event-replay validation

These flags reconstruct balances by replaying the raw event log and compare the reconstruction to the database snapshot and the on-chain value.

### `--calculate-from-events <USER_ADDRESS>`

Reconstructs a user's token balance from money market events and compares the reconstruction to the on-chain balance:

1. Resolves the reserve via the supplied token flag and looks up the relevant current index (liquidity index for aToken/reserve, variable borrow index for debt token).
2. Fetches all events for the user, replays them to produce the scaled balance, then derives the real balance using the current index.
3. Compares against on-chain balance **at the block of the last event** (the meaningful comparison) and at the **latest block** (informational).
4. Prints a verdict: perfect / `< 0.01%` / `< 1%` / `>= 1%` mismatch.

Requires exactly one of `--reserve-token`, `--a-token`, or `--debt-token`.

```bash
cargo run -- --calculate-from-events 0xuser... --reserve-token 0xtoken...
cargo run -- --calculate-from-events 0xuser... --a-token 0xatoken...
cargo run -- --calculate-from-events 0xuser... --debt-token 0xdebt...
```

### `--calculate-from-events-reserve <RESERVE_ADDRESS>`

Reserve-scoped event-replay reconstruction: iterates **every user with a position in the reserve** (both supply and borrow sides by default) and produces a **three-way** scaled balance comparison for each user.

The three columns are:

| Column | Source | What it represents |
|---|---|---|
| **DB Events Scaled** | This tool's replay of `money_market_events` | The scaled balance our replay reconstructs from raw events |
| **Position Scaled** | `user_positions.positions[].aTokenBalance` / `.variableDebtTokenBalance` | The scaled balance the SODAX backend derived from events and stored in `user_positions` |
| **Chain Scaled** | `scaledBalanceOf(user)` pinned to `last_event_block` | The scaled balance the chain actually has |

This isolates two independent integrity gates:

- **DB Events Scaled vs Chain Scaled** (the primary `Diff%` and `Verdict` columns) — does the event log we ingest match what the chain applied? Failures here mean missed / wrong events in `money_market_events`.
- **DB Events Scaled vs Position Scaled** (eyeball comparison) — does the backend's `user_positions` derivation logic agree with our replay of the same events? Failures here mean a bug in how the backend computes `user_positions` from `money_market_events`.

The handler fetches the full token-event stream **once per side** (one query for the aToken, one for the variable-debt token) via `find_token_events_sorted`. That stream is shared by `Arc` across every user's replay. For each user × selected side, the handler then:

1. Runs `process_user_token_events` against the **full token-event stream**. The function filters the deltas to the target user internally, but processes every event in block / logIndex order first — so `last_index` is updated by any mint/burn by *any* user before our user's transfer events land. (Earlier per-user-only fetches missed cross-user mints that defined the pool's liquidity index at transfer time, which caused transfers-as-first-event to credit an inflated scaled balance.)
2. The result is the **scaled** balance (raw value before liquidity / variable-borrow index is applied) plus the user's `last_event_block`.
3. Looks up the user's `user_positions` document and pulls the scaled balance for this reserve + side.
4. Calls `scaledBalanceOf(user)` on the chain, **pinned to the user's last event block** (alloy's `.block(N)` historical call). This sidesteps the liquidity / variable-borrow index entirely — no f64 conversion, no question of whether the index was queried at the same block as the balance, no drift from interest accrual between snapshots. If `last_event_block == 0` (no matching events found for the user in the token stream), the row is classified `ERROR` rather than degrading to an unpinned latest-block comparison.
5. Compares `db_scaled` to `chain_scaled` and classifies the row: `PERFECT` / `EXCELLENT` (<0.01%) / `MINOR` (<1%) / `SIGNIFICANT` (≥1%) / `ERROR`.

> **Why scaled, not real?** Real balances on Aave-style markets are `scaledBalance × index / RAY` where `index` keeps moving as interest accrues. Comparing real-vs-real requires both sides to agree on the index at exactly the same block; comparing scaled-vs-scaled is a direct integer equality check that needs no index at all. Both `user_positions` and `scaledBalanceOf` are stored / computed in scaled units, so the three-way comparison is apples-to-apples.

Suppliers are pulled from `reserve_tokens.suppliers` and borrowers from `reserve_tokens.borrowers`. Within a single side, up to 10 user replays run in parallel; the two sides are processed sequentially (supply then borrow), so peak concurrency across the whole command is ~10 — not 20. Row order in the output matches the input user list (the implementation uses an ordered `buffered` stream).

**Output modes:**
- Default: one compact table per side (User, DB Events Scaled, Position Scaled, Chain Scaled, Diff%, Verdict) plus a per-side summary count. `Position Scaled` shows `-` if the user has no `user_positions` document or no entry for this reserve.
- `--verbose`: prints the full per-user replay block (matching `--calculate-from-events`) for every user. Useful for debugging a small reserve; noisy on large ones. Replays run **sequentially** in this mode so each user's block stays contiguous (rather than interleaved across concurrent tasks).
- `--json`: emits the full result (per-user rows + summary, per side) as a single JSON document for downstream tooling. Each user row has `dbScaled`, `positionScaled` (string or `null`), `positionError` (string or `null`), `chainScaled`, `diff` (all u128 as strings to survive JSON precision), `percentage` (f64, display), `lastEventBlock`, `verdict`, and `error`. `positionScaled` and `positionError` are mutually exclusive — exactly one is populated.
- `--verbose` and `--json` are mutually exclusive.

**Performance:**
- Events are fetched **once per side** (one query for the aToken, one for the variable-debt token) and shared across every user via `Arc`. This is N=2 DB queries for events regardless of user count, and — crucially — captures mints/burns by *every* user for the token. `process_user_token_events` filters to the target user internally, but sees the full stream first, so `last_index` is always up-to-date by the time a user's transfer lands. (Earlier per-user-only fetches missed cross-user mints that defined the pool's liquidity index at transfer time, which produced inflated scaled balances for users whose first event for a token was a transfer.)
- `user_positions` are prefetched once per unique user (case-insensitive union of `suppliers` and `borrowers`) with bounded fan-out (`buffered(10)`).
- Non-verbose replays use bounded concurrency (10 in flight). Supply and borrow sides are processed sequentially, so peak concurrency is ~10 across the command. Verbose runs sequentially.

**Verdict math:**
- The verdict bucket is computed from `diff` and `chain_scaled` with u128 integer arithmetic, so it's exact regardless of balance magnitude. `percentage` is f64 for display only.
- When `chain_scaled == 0` but `diff > 0` (DB replay produced a balance the chain doesn't have), the row is classified `SIGNIFICANT`. This aligns with `EntryState::new` in `structs.rs` and **differs from `--calculate-from-events`**, which reports such cases as 0% / "Excellent". If you need to cross-reference, treat this handler as authoritative.

**Side selection:**
- Default: process both supply and borrow.
- `--a-token-only`: skip the borrow side.
- `--debt-token-only`: skip the supply side.
- `--a-token-only` and `--debt-token-only` are mutually exclusive.

```bash
# Both sides
cargo run -- --calculate-from-events-reserve 0xreserve...

# Supply only
cargo run -- --calculate-from-events-reserve 0xreserve... --a-token-only

# Debt only
cargo run -- --calculate-from-events-reserve 0xreserve... --debt-token-only

# Verbose per-user output
cargo run -- --calculate-from-events-reserve 0xreserve... --verbose

# Machine-readable
cargo run -- --calculate-from-events-reserve 0xreserve... --json
```

### `--calculate-from-events-reserve-all`

Market-wide variant of `--calculate-from-events-reserve`: iterates **every reserve** returned by `find_all_reserves()` and runs the same per-user × per-side scaled-vs-scaled comparison for each. Designed for a regular health check across the whole money market — no need to keep a hand-maintained list of reserve addresses in sync.

For each reserve the output mirrors the single-reserve flag's compact-table format (one supply table + one borrow table, with the same `Position Scaled` column and `Verdict` classification). At the end, the handler prints a **market-wide summary**: total reserves processed, aggregate verdict bucket counts (separately per side), and the list of reserves that contributed any `SIGNIFICANT` (❌), `MINOR` (⚠️), or `ERROR` (‼️) row on either side.

**Per-reserve behavior:**
- Reuses the existing replay helpers (`replay_side`, `print_replay_table`, `rows_summary_json`, `count_buckets`) — no separate code path for the per-reserve work.
- `--a-token-only` / `--debt-token-only` apply across every reserve in the scan, exactly as for the single-reserve flag.
- **Reserve-level failures don't abort the run.** If `find_token_events_sorted` fails for one side of one reserve (e.g. RPC hiccup, missing collection entry), every user on that side is counted as `ERROR` in both the per-reserve summary line and the market-wide aggregate, and the scan moves on to the next reserve. The fetch-error message is printed in place of the per-user table and surfaces as `fetchError` on the side in `--json` output.

**`--verbose` is rejected.** Running the verbose per-event replay across every reserve produces unusable amounts of output; verbose's value is in single-reserve debugging via the existing `--calculate-from-events-reserve <ADDR> --verbose` form.

**Concurrency.** Reserves are processed **sequentially**, one at a time. Each reserve already fans out up to 10 user replays in parallel internally; stacking reserve-level concurrency on top would multiply DB / RPC load. Peak in-flight concurrency stays at ~10 across the whole command — same as the single-reserve flag.

**Performance.**
- `user_positions` are prefetched once for the **union** of every selected side's user list across every reserve (case-insensitive). Users active in multiple reserves (the common case) hit the DB once instead of once-per-reserve. Bounded fan-out via `buffered(10)`, matching the existing prefetch helper.
- Token events are still fetched once per reserve × selected side via `find_token_events_sorted`.

**Output modes:**
- Default (text): per-reserve section (header + supply table + borrow table + per-side summary line) for each reserve, followed by the market-wide summary block.
- `--json`: emits a single JSON object `{ "reserves": [ ...per-reserve docs..., shaped exactly like --calculate-from-events-reserve --json output... ], "summary": { ...market-wide aggregate... } }`. Each per-reserve doc carries an optional `fetchError` string on `supply` / `borrow` when that side's event fetch failed. The market-wide `summary` includes `reservesProcessed`, `mode`, per-side `BucketCounts` (or `null` if the side was skipped), and the `reservesWithSignificant` / `reservesWithMinor` / `reservesWithErrors` lists (each an array of `{ "symbol", "reserve" }`).

```bash
# Compact market-wide scan (both sides)
cargo run -- --calculate-from-events-reserve-all

# Supply side only across every reserve
cargo run -- --calculate-from-events-reserve-all --a-token-only

# Debt side only across every reserve
cargo run -- --calculate-from-events-reserve-all --debt-token-only

# Machine-readable, for ingest into monitoring / CI
cargo run -- --calculate-from-events-reserve-all --json
```

### `--validate-from-events <USER_ADDRESS>`

3-way validation for one user across all positions (or one position when filtered):

- **events** — replayed from `money_market_events`
- **DB** — stored `user_positions` snapshot
- **chain** — on-chain balance

Reports pairwise diffs and percentages for `events_vs_chain`, `db_vs_chain`, and `events_vs_db`. Only `--reserve-token` may be combined (optional, to filter to one reserve).

```bash
cargo run -- --validate-from-events 0xuser...
cargo run -- --validate-from-events 0xuser... --reserve-token 0xtoken...
```

### `--validate-from-events-all`

Runs `--validate-from-events` across every user with bounded concurrency (10 in parallel). Standalone.

```bash
cargo run -- --validate-from-events-all
```

## Partner asset validation

### `--validate-partner-asset`

Recomputes `partner_asset` aggregates from `solver_volume` (the authoritative source) and reports drift between the stored aggregates and the recomputed values. For each `(receiver, asset, chainId) × outputToken` row it reports:

- `txCount`, `totalFeeIn`, `totalVolumeOut` — stored vs. computed
- Ratios per metric; rows within `±threshold` of 1.0 are suppressed from the printed table
- Status: `OK`, `DRIFT`, `MISSING_IN_STORED`, or `EXTRA_IN_STORED`

Optional companions: `--partner`, `--json`, `--threshold`.

```bash
cargo run -- --validate-partner-asset
cargo run -- --validate-partner-asset --partner 0xpartner...
cargo run -- --validate-partner-asset --threshold 0
cargo run -- --validate-partner-asset --json
```

### `--partner <PARTNER_ADDRESS>`

Restricts `--validate-partner-asset` to a single receiver address. Only valid alongside `--validate-partner-asset`.

### `--json`

Emit `--validate-partner-asset` output as JSON instead of a table. Only valid alongside `--validate-partner-asset`.

### `--threshold <PCT>`

Drift tolerance for `--validate-partner-asset`: rows whose stored/computed ratios are all within `±PCT` of 1.0 are suppressed from the table. Default `0.0001`. Only valid alongside `--validate-partner-asset`.

## Modifiers

### `--scaled`

Switches validation flags from comparing real balances to comparing scaled (raw) balances.

- **Real balance** (default) = `scaled_balance × current_index / 10^27` — what a user can actually withdraw or owes.
- **Scaled balance** = the raw value stored in `user_positions` before index application.

Use `--scaled` to detect drift in the underlying scaled values without the noise introduced by index updates between snapshot time and validation time.

Combines with: `--validate-user-supply`, `--validate-user-borrow`, `--validate-token-supply`, `--validate-token-borrow`, `--validate-user-all`, `--validate-users-all`, `--validate-token-all`, `--validate-all`.

## Combination rules

Enforced by the parser in [`src/cli.rs`](../src/cli.rs):

**Standalone (cannot combine with anything):**
`--help`, `--last-block`, `--all-tokens`, `--orderbook`, `--timestamp-coverage`, `--get-all-users`, `--get-all-reserves`, `--get-all-a-token`, `--get-all-debt-token`, `--validate-all-reserve-indexes`, `--validate-from-events-all`. The argument-bearing variants `--validate-timestamps`, `--get-token-events`, `--get-user-events`, `--validate-reserve-indexes` are similarly standalone (only their own arg).

> Note: the parser will accept `--validate-from-events-all --scaled` without an error, but the dispatch in `main.rs` always calls the non-scaled handler — `--scaled` is silently ignored for this flag.

**Combinable only with `--scaled`:**
`--validate-users-all`, `--validate-user-all`, `--validate-token-all`, `--validate-all`.

**Token-flag mutual exclusion:**
You cannot mix `--reserve-token`, `--a-token`, and `--debt-token` in a single invocation.

**Companion-required:**
- `--balance-of`, `--user-position`, `--calculate-from-events` → require one of `--reserve-token` / `--a-token` / `--debt-token`.
- `--inspect-user-position` → requires one of `--a-token` / `--debt-token` (no `--reserve-token`).
- `--validate-user-supply`, `--validate-user-borrow`, `--validate-token-supply`, `--validate-token-borrow` → require `--reserve-token`.
- `--block` → only valid with `--balance-of`.

**Partner-asset subgroup:**
- `--validate-partner-asset` accepts `--partner`, `--json`, `--threshold` (all optional).
- `--partner` and `--threshold` are rejected if used without `--validate-partner-asset`.

**Reserve event-replay subgroup:**
- `--calculate-from-events-reserve` accepts `--a-token-only`, `--debt-token-only`, `--verbose`, `--json` (all optional).
- `--calculate-from-events-reserve-all` accepts `--a-token-only`, `--debt-token-only`, `--json` (all optional). `--verbose` is **rejected** because verbose output across every reserve is impractical — drill into a single reserve with `--calculate-from-events-reserve <ADDR> --verbose` instead.
- `--a-token-only` and `--debt-token-only` are mutually exclusive (applies to both flags).
- `--verbose` and `--json` are mutually exclusive (single-reserve flag only).
- `--a-token-only` / `--debt-token-only` are rejected if used without one of the reserve-event-replay flags. `--verbose` is rejected if used without `--calculate-from-events-reserve`.

**Shared modifier:**
- `--json` is valid with `--validate-partner-asset`, `--calculate-from-events-reserve`, or `--calculate-from-events-reserve-all`; rejected otherwise.

**Event-replay companions:**
- `--validate-from-events` accepts `--reserve-token` (optional). No other combinations.

## Reports

By default, every command (except `--help`) writes a report file to `reports/report_<unix_timestamp>.txt`. The path is printed on exit:

```
Report saved to: reports/report_1747200123.txt
```

Pass `--no-report` to skip this. The directory must be writable; the `reports/` directory exists in the repo and is gitignored by default.

**Caveat — what gets captured:** report files capture only lines emitted through the internal `output!` macro. Handlers that print with `println!` (currently `handle_validate_from_events` and `handle_validate_from_events_all`, i.e. `--validate-from-events` and `--validate-from-events-all`) write to stdout only, so their report files will be empty / incomplete relative to terminal output. Treat the report file as a best-effort transcript, not a guaranteed mirror of stdout.
