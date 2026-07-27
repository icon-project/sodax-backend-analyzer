# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rust CLI tool for analyzing and validating SODAX backend data. It queries a local MongoDB instance (containing SODAX backend data) and validates it against on-chain state via EVM RPC calls. Requires a running local MongoDB and an accessible RPC endpoint.

## Build & Development Commands

```bash
cargo build                    # Build (debug)
cargo build --release          # Build (release)
cargo run -- --help            # Run with flags
cargo check                    # Check compilation
cargo clippy                   # Lint
cargo fmt                      # Format code
cargo test                     # Run all tests (requires running MongoDB)
cargo test --test mongodb_integration_tests   # Run specific test file
cargo test -- --nocapture      # Tests with stdout visible
```

Make targets: `make build`, `make lint`, `make fmt`, `make test`, `make clean`

## Code Formatting

Uses `rustfmt.toml`: 2-space indentation (`tab_spaces = 2`), max width 100, imports are NOT reordered (`reorder_imports = false`).

## Pre-commit Hooks

cargo-husky runs `cargo check` and `cargo clippy` on pre-commit.

## Architecture

**CLI flow:** `main.rs` → `cli::parse_args()` → returns `Vec<Flag>` → `main` dispatches to handler functions.

- **`structs.rs`** — `Flag` enum defines all CLI flags (some carry String/u64 values). Also defines validation result types (`EntryState`, `UserPositionValidation`, `ReserveEntryState`) and `Collections` (MongoDB collection name constants).
- **`cli.rs`** — Hand-rolled argument parser. Flags are standalone or combinable (e.g., `--balance-of <addr> --reserve-token <addr>`). Validation rules enforce which flags can combine.
- **`handlers.rs`** — One handler function per CLI command. Handlers orchestrate DB queries, EVM calls, and validation. Bulk operations use `tokio` with semaphore-based concurrency limiting (10 concurrent users, 5 positions per user).
- **`validators.rs`** — Validation logic comparing database values against on-chain state. Supports both real balances (index-adjusted) and scaled balances (raw).
- **`db.rs`** — MongoDB queries against collections: `reserve_tokens`, `user_positions`, `orderbook`, `money_market_events`, `wallet_factory_events`, `intent_events`, etc.
- **`evm.rs`** — On-chain calls via `alloy` crate (block numbers, token balances, contract reads).
- **`models.rs`** — MongoDB document schemas (serde deserialization).
- **`config.rs`** — Loads `.env` vars (`MONGO_USER`, `MONGO_PASSWORD`, `MONGO_HOST`, `MONGO_PORT`, `MONGO_DB`, `RPC_PROVIDER`).
- **`balance_calculator.rs`** — Balance computation from event history.
- **`functions.rs`** — Flag extraction helpers (e.g., extracting address from a `Flag` variant).
- **`helpers.rs`** — Utility functions.
- **`constants.rs`** — Global constants and help text.

## Key Patterns

- The `Flag` enum is the central dispatch mechanism — adding a new CLI command means adding a variant to `Flag`, parsing logic in `cli.rs`, a handler in `handlers.rs`, and a dispatch branch in `main.rs`.
- Balance validation compares `database_amount` vs `on_chain_amount` and computes difference/percentage via `EntryState`.
- The `--scaled` flag toggles between real balance validation (applies liquidity/borrow index) and raw scaled balance validation.
- MongoDB collection names are centralized in `Collections::new()` in `structs.rs`.

## Environment

Requires a `.env` file with: `MONGO_USER`, `MONGO_PASSWORD`, `MONGO_HOST`, `MONGO_PORT`, `MONGO_DB`, `RPC_PROVIDER`.

## Testing

All tests are integration tests requiring a running MongoDB with SODAX data. Test files are in `tests/` with shared utilities in `tests/common.rs`.

## SODAX Backend Data Model Reference

Detailed reference docs for the backend's MongoDB data model live in `docs/sodax-backend/`:

| File | Contents |
|------|----------|
| [`COLLECTIONS.md`](docs/sodax-backend/COLLECTIONS.md) | Every collection with exact field names, types, indexes, discriminator patterns |
| [`ENUMS.md`](docs/sodax-backend/ENUMS.md) | All enum values (event types, chain IDs, statuses, intervals) |
| [`DATA_FLOW.md`](docs/sodax-backend/DATA_FLOW.md) | Architecture diagram, write ownership table, domain rules |

**Key gotchas:**

- **Collection names are snake_case** in MongoDB (e.g., `intent_events`, `user_positions`). Verify `Collections::new()` in `structs.rs` matches — any camelCase names there (like `"intentEvents"`) may be legacy bugs.
- **All BigInt fields are stored as `Decimal128`** in MongoDB. Deserialize via `bson::Decimal128` then convert to `u128`/`U256`.
- **Event collections use discriminators** on the `eventType` field — different variants have different fields present. Always filter by `eventType` when querying.
- **`intentId` is NOT unique** — always use `intentHash` as the unique intent identifier.
- **Scaled balance math**: `user_positions` stores scaled (not real) balances. Real balance = `scaledBalance × currentIndex / 10^27`.

<!-- BEGIN LOCAL DEV RESOURCE COMMANDS -->
## Resource-safe commands for AI agents (shared dev server)

> **Read `~/CLAUDE.md` → "Shared dev-server resource policy" first.** It defines `dev-status`, the go/no-go thresholds (RAM < 8 GiB, swap > 8 GiB, load > 20, another heavy job running) and the `heavy-run` lock. This section only maps that policy onto this repo.
>
> These are **instructions for which commands an interactive AI agent runs on this dev box.** They change nothing for developers or CI, and **no `Cargo.toml`, `Makefile`, `rustfmt.toml`, or the cargo-husky hook may be edited to enforce them.**

**Package manager / toolchain:** **Cargo** (Rust). No Node, no npm/pnpm in this repo.
**Test runner:** **`cargo test` — no Vitest, no Jest.** All tests are integration tests in `tests/` and **require a running MongoDB with SODAX data** (see [Testing](#testing) above).
**Existing commands** (see [Build & Development Commands](#build--development-commands) above — unchanged): `cargo build`, `cargo build --release`, `cargo check`, `cargo clippy`, `cargo fmt`, `cargo test`, and `make build` / `make lint` / `make fmt` / `make test` / `make clean`.

### Cargo fans out across all 20 cores by default

`cargo` uses one codegen job per core, and `cargo test` runs test binaries on one thread per core on top of that. Cap both **on the command line** — never in `Cargo.toml` or `.cargo/config.toml`:

- `-j 4` — at most 4 build jobs
- `-- --test-threads=4` — at most 4 concurrent tests

### Targeted first — run these directly, no `heavy-run`

```bash
cargo check -j 4                                                  # fast, no codegen
cargo fmt                                                         # trivial
cargo test --test mongodb_integration_tests -j 4 -- --test-threads=4   # one test file
cargo test some_test_name -j 4 -- --test-threads=4                # one test
```

### Repository-wide — `heavy-run`, and tell the user first

```bash
heavy-run timeout 20m cargo build -j 4
heavy-run timeout 30m cargo build --release -j 4     # release codegen is the heaviest job here
heavy-run timeout 20m cargo clippy -j 4
heavy-run timeout 20m cargo test -j 4 -- --test-threads=4
```

Prefer the `cargo …` forms above over `make build` / `make lint` / `make test` when running as an agent — the `make` targets don't take `-j`, so they'll fan out across all 20 cores.

### ⚠️ `git commit` triggers a build in this repo

cargo-husky's pre-commit hook runs `cargo check` **and** `cargo clippy` (see [Pre-commit Hooks](#pre-commit-hooks)). On a cold `target/` that is a full build. Take the lock:

```bash
heavy-run timeout 20m git commit -m "…"
```
<!-- END LOCAL DEV RESOURCE COMMANDS -->
