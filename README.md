# SODAX Backend Analyzer

A Rust CLI tool for analyzing database data for the SODAX backend. This tool provides easy access to MongoDB data for reserve tokens, user positions, and blockchain information.

## 🚀 Features

- **Reserve Token Analysis** - Query reserve token data by various token addresses
- **User Position Tracking** - Get user position data by wallet address
- **Orderbook** - Get the orderbook of pending intents
- **Blockchain Integration** - Get latest block numbers and token balances
- **MongoDB Integration** - Direct connection to MongoDB database
- **CLI Interface** - Simple command-line interface for data queries
- **EVM Support** - Interact with Ethereum-compatible blockchains
- **Data Validation** - Comprehensive validation of database vs on-chain data
- **Scaled Balance Validation** - Validate raw database values against on-chain scaled balances using the `--scaled` flag
- **Bulk Operations** - Validate all reserves and user positions at once with parallel processing
- **Data Fetching** - Get all users, reserves, aTokens, and debt tokens from the database
- **Event Retrieval** - Get events for specific tokens and users
- **Index Validation** - Validate liquidity and borrow indexes for reserves
- **Position Inspection** - Inspect user balance history and detect missed events
- **Error Handling** - Robust error handling with graceful degradation

## 📋 Prerequisites

- **Rust** (latest stable version)
- **Local MongoDB instance** running on your machine with a copy of the SODAX backend database
- **Environment variables** configured (see Configuration section)

> **Important**: This tool requires a local MongoDB instance with the SODAX backend database. You cannot use this tool without having the database running locally on your machine.

## 🛠️ Installation

1. **Clone the repository:**
   ```bash
   git clone <repository-url>
   cd sodax-backend-analizer
   ```

2. **Build the project:**
   ```bash
   cargo build --release
   ```

3. **Set up environment variables** (see Configuration section)

## ⚙️ Configuration

Create a `.env` file in the project root with the following variables:

```env
# MongoDB Configuration
MONGO_USER=your_mongo_username
MONGO_PASSWORD=your_mongo_password
MONGO_HOST=your_mongo_host
MONGO_PORT=27017
MONGO_DB=your_database_name

# RPC Provider Configuration
RPC_PROVIDER=https://rpc.soniclabs.com/
```

### Environment Variables

| Variable | Description | Required | Example |
|----------|-------------|----------|---------|
| `MONGO_USER` | MongoDB username | Yes | `admin` |
| `MONGO_PASSWORD` | MongoDB password | Yes | `your_password` |
| `MONGO_HOST` | MongoDB host address | Yes | `127.0.0.1` or `localhost` |
| `MONGO_PORT` | MongoDB port number | Yes | `27017` |
| `MONGO_DB` | Database name | Yes | `sodax_backend` |
| `RPC_PROVIDER` | Ethereum-compatible RPC endpoint URL | Yes | `https://rpc.soniclabs.com/` |

> **Note**: The `RPC_PROVIDER` is used for all on-chain queries including balance validations, block information, and contract interactions. Make sure the RPC endpoint is accessible and supports the network you're validating against.

## 🎯 Usage

### Inspecting User Positions

The `--inspect-user-position` flag allows you to audit a user's balance history for a specific token and detect any missed events:

```bash
# Inspect aToken position
cargo run -- --inspect-user-position <USER_ADDRESS> --a-token <ATOKEN_ADDRESS>

# Inspect debt token position
cargo run -- --inspect-user-position <USER_ADDRESS> --debt-token <DEBT_TOKEN_ADDRESS>
```

**What it does:**
1. Fetches all relevant events for the user from the `money_market_events` collection
2. Extracts event IDs from the user's balance history in the `user_positions` collection
3. Compares the two lists to identify any missing events
4. Outputs a JSON report with statistics and details of missed events

**Output format:**
```json
{
  "user": "0x...",
  "tokenAddress": "0x...",
  "tokenType": "aToken" or "debtToken",
  "eventsOnMoneyMarketEventCollection": 3573,
  "eventsOnUserBalanceHistory": 3573,
  "eventsMissedCount": 0,
  "missedEvents": []
}
```

**Use cases:**
- Verify balance history integrity
- Debug discrepancies between events and positions
- Audit event processing completeness
- Identify data synchronization issues

### Understanding Scaled vs Real Balances

The tool supports two types of balance validation:

- **Real Balances** (default): Calculated by applying current liquidity/borrow indices to scaled balances
  - Formula: `real_balance = scaled_balance * liquidity_index / RAY`
  - This shows the actual token amounts users can withdraw/repay

- **Scaled Balances** (with `--scaled` flag): Raw values stored in the database before index application
  - These are the base values that get updated by liquidity/borrow indices over time
  - Use `--scaled` to validate the raw database values against on-chain scaled balances

### Basic Commands

```bash
# Show help
cargo run -- --help

# Get all reserve tokens
cargo run -- --all-tokens

# Get latest block number
cargo run -- --last-block

# Get all orderbook data
cargo run -- --orderbook

# Get all user addresses
cargo run -- --get-all-users

# Get all reserve tokens with addresses and symbols
cargo run -- --get-all-reserves

# Get all aToken addresses and symbols
cargo run -- --get-all-a-token

# Get all debt token addresses and symbols
cargo run -- --get-all-debt-token

# Get reserve token data by reserve address
cargo run -- --reserve-token <RESERVE_ADDRESS>

# Get reserve token data by aToken address
cargo run -- --a-token <ATOKEN_ADDRESS>

# Get reserve token data by debt token address
cargo run -- --debt-token <DEBT_TOKEN_ADDRESS>

# Get user position data by wallet address
cargo run -- --user-position <WALLET_ADDRESS>

# Get token balance for a user (requires token type flag)
cargo run -- --balance-of <USER_ADDRESS> --reserve-token <TOKEN_ADDRESS>

# Get events for a specific token
cargo run -- --get-token-events <TOKEN_ADDRESS>

# Get events for a specific user
cargo run -- --get-user-events <USER_ADDRESS>

# Inspect user position for a specific token (checks for missed events)
cargo run -- --inspect-user-position <USER_ADDRESS> --a-token <ATOKEN_ADDRESS>
cargo run -- --inspect-user-position <USER_ADDRESS> --debt-token <DEBT_TOKEN_ADDRESS>

# Validate reserve indexes for a specific reserve
cargo run -- --validate-reserve-indexes <RESERVE_ADDRESS>

# Validate indexes for all reserves
cargo run -- --validate-all-reserve-indexes

# Individual validation (real balances)
cargo run -- --validate-user-supply <USER_ADDRESS> --reserve-token <RESERVE_ADDRESS>
cargo run -- --validate-user-borrow <USER_ADDRESS> --reserve-token <RESERVE_ADDRESS>
cargo run -- --validate-token-supply --reserve-token <RESERVE_ADDRESS>
cargo run -- --validate-token-borrow --reserve-token <RESERVE_ADDRESS>

# Individual validation (scaled balances)
cargo run -- --validate-user-supply <USER_ADDRESS> --reserve-token <RESERVE_ADDRESS> --scaled
cargo run -- --validate-user-borrow <USER_ADDRESS> --reserve-token <RESERVE_ADDRESS> --scaled
cargo run -- --validate-token-supply --reserve-token <RESERVE_ADDRESS> --scaled
cargo run -- --validate-token-borrow --reserve-token <RESERVE_ADDRESS> --scaled

# Bulk validation (real balances)
cargo run -- --validate-user-all <USER_ADDRESS>
cargo run -- --validate-users-all
cargo run -- --validate-token-all
cargo run -- --validate-all

# Bulk validation (scaled balances)
cargo run -- --validate-user-all <USER_ADDRESS> --scaled
cargo run -- --validate-users-all --scaled
cargo run -- --validate-token-all --scaled
cargo run -- --validate-all --scaled

### Examples

```bash
# Get all reserve tokens
cargo run -- --all-tokens

# Get latest block number
cargo run -- --last-block

# Get all orderbook data
cargo run -- --orderbook

# Query a specific reserve token
cargo run -- --reserve-token 0x1234567890123456789012345678901234567890

# Query by aToken address
cargo run -- --a-token 0x5c50cf875aebad8d5ba548f229960c90b1c1f8c3

# Query by debt token address
cargo run -- --debt-token 0x5c50cf875aebad8d5ba548f229960c90b1c1f8c3

# Get all user addresses
cargo run -- --get-all-users

# Get all reserve tokens
cargo run -- --get-all-reserves

# Get all aToken addresses
cargo run -- --get-all-a-token

# Get all debt token addresses
cargo run -- --get-all-debt-token

# Get events for a specific token
cargo run -- --get-token-events 0x1234567890abcdef...

# Get events for a specific user
cargo run -- --get-user-events 0xuser123...

# Inspect user position for missed events
cargo run -- --inspect-user-position 0xuser123... --a-token 0x1234567890abcdef...
cargo run -- --inspect-user-position 0xuser123... --debt-token 0x1234567890abcdef...

# Validate reserve indexes
cargo run -- --validate-reserve-indexes 0x1234567890abcdef...

# Validate all reserve indexes
cargo run -- --validate-all-reserve-indexes

# Get user balance for a specific token
cargo run -- --balance-of 0xuser123... --reserve-token 0xtoken456...

# Validate user supply and borrow positions (real balances)
cargo run -- --validate-user-supply 0xuser123... --reserve-token 0xtoken456...
cargo run -- --validate-user-borrow 0xuser123... --reserve-token 0xtoken456...

# Validate user supply and borrow positions (scaled balances)
cargo run -- --validate-user-supply 0xuser123... --reserve-token 0xtoken456... --scaled
cargo run -- --validate-user-borrow 0xuser123... --reserve-token 0xtoken456... --scaled

# Validate token total supply and borrow (real balances)
cargo run -- --validate-token-supply --reserve-token 0xtoken456...
cargo run -- --validate-token-borrow --reserve-token 0xtoken456...

# Validate token total supply and borrow (scaled balances)
cargo run -- --validate-token-supply --reserve-token 0xtoken456... --scaled
cargo run -- --validate-token-borrow --reserve-token 0xtoken456... --scaled

# Bulk validation examples (real balances)
cargo run -- --validate-user-all 0xuser123...
cargo run -- --validate-users-all
cargo run -- --validate-token-all
cargo run -- --validate-all

# Bulk validation examples (scaled balances)
cargo run -- --validate-user-all 0xuser123... --scaled
cargo run -- --validate-users-all --scaled
cargo run -- --validate-token-all --scaled
cargo run -- --validate-all --scaled
```

## 🏗️ Project Structure

```
sodax-backend-analizer/
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library entry point and re-exports
│   ├── cli.rs               # CLI argument parsing and help
│   ├── config.rs            # Configuration management
│   ├── db.rs                # Database operations
│   ├── evm.rs               # EVM blockchain integration
│   ├── handlers.rs          # CLI command handlers
│   ├── helpers.rs           # Helper functions
│   ├── functions.rs         # Flag extraction and utility functions
│   ├── validators.rs        # Data validation logic
│   ├── constants.rs         # Global constants and help message
│   ├── structs.rs           # Data structures and enums
│   └── models.rs            # Data models and MongoDB schemas
├── tests/
│   ├── common.rs            # Common test utilities
│   ├── evm_integration_tests.rs
│   ├── general_integration_tests.rs
│   └── mongodb_integration_tests.rs
├── Cargo.toml               # Rust project configuration
├── Cargo.lock               # Dependency lock file
├── Makefile                 # Build and development commands
├── rustfmt.toml             # Rust code formatting configuration
├── .gitignore               # Git ignore patterns
├── TODO.md                  # Development roadmap and tasks
└── README.md                # Project documentation
```

## 🧪 Testing

### Run Tests
```bash
# Run all tests
cargo test

# Run only integration tests
cargo test --test mongodb_integration_tests

# Run with verbose output
cargo test -- --nocapture
```

### Test Requirements
- MongoDB instance must be running
- Environment variables must be configured
- Database should contain valid data from the SODAX backend

## 🔧 Development

### Code Quality Checks
```bash
# Check compilation
cargo check

# Run linter
cargo clippy

# Format code
cargo fmt

# Run all checks
cargo check && cargo clippy -- -D warnings
```

### Git Hooks
The project uses Git hooks to ensure code quality:

- **Pre-commit**: Runs `cargo check` and `cargo clippy`
- **Automatic setup**: Hooks are configured via cargo-husky

## ⚡ Performance & Concurrency

The tool uses intelligent concurrency limiting to balance performance with resource constraints:

### Concurrency Limits

- **User Validation**: Maximum 10 users validated concurrently
- **Position Validation**: Maximum 5 positions per user validated concurrently
- **Total Concurrent Operations**: ~150 file descriptors used at peak

### Why These Limits?

When validating all users with `--validate-all` or `--validate-users-all`, the tool:
1. Connects to MongoDB for each user's position data
2. Makes RPC calls to the blockchain for on-chain validation
3. Each operation opens file descriptors for network connections

Without limits, validating hundreds of users with multiple positions each would:
- Open thousands of simultaneous connections
- Exceed the OS file descriptor limit (typically 1024 on Linux)
- Result in "Too many open files" errors

### Tuning Performance

You can adjust these limits based on your system resources:

**In `src/handlers.rs` (line ~734)**:
```rust
let max_concurrent_users = 10;  // Increase for faster processing
```

**In `src/validators.rs` (line ~211)**:
```rust
let max_concurrent_positions = 5;  // Increase for faster processing
```

**Safe Formula**:
```
max_concurrent_users × max_concurrent_positions × 3 < (ulimit -n / 2)
```

Example: With `ulimit -n` of 1024, use `10 × 5 × 3 = 150` (safe!)

## 🔍 Data Sources

### MongoDB Collections
The tool connects to the following MongoDB collections:

- `reserve_tokens` - Reserve token data
- `user_positions` - User position data
- `orderbook` - Orderbook information
- `money_market_events` - Money market events
- `wallet_factory_events` - Wallet factory events
- `intent_events` - Intent events

## 🆘 Troubleshooting

### Common Issues

**Database Connection Failed**
- Verify MongoDB is running
- Check environment variables
- Ensure network connectivity

**Blockchain Connection Failed**
- Verify `RPC_PROVIDER` is set correctly in your `.env` file
- Ensure the RPC endpoint is accessible and supports the target network
- Check network connectivity and firewall settings
- Test the RPC endpoint manually (e.g., using `curl` or a web browser)
- Ensure valid Ethereum addresses are provided

**Tests Failing**
- Ensure MongoDB instance is available
- Check test data exists in database
- Verify environment configuration

**Compilation Errors**
- Run `cargo check` for detailed error messages
- Ensure all dependencies are up to date
- Check Rust toolchain version

**CLI Validation Errors**
- Check flag combinations (see help for restrictions)
- Ensure required arguments are provided
- Verify address formats are valid Ethereum addresses

**"Too many open files" Error**
- The tool uses concurrency limits to prevent file descriptor exhaustion
- Default limits: 10 concurrent users, 5 concurrent positions per user
- If you still encounter this error:
  - Check your system's file descriptor limit: `ulimit -n`
  - Increase the limit if needed: `ulimit -n 4096`
  - Or adjust concurrency limits in the source code:
    - `src/handlers.rs` line ~734: `max_concurrent_users`
    - `src/validators.rs` line ~211: `max_concurrent_positions`

---

**Important Notes**:
- This tool requires a **local MongoDB instance** running on your machine with the SODAX backend database
- You cannot use this tool without having the database running locally
- Internet connectivity is required for blockchain RPC calls
- Make sure your environment is properly configured before use
