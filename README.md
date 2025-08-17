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
MONGO_USER=your_mongo_username
MONGO_PASSWORD=your_mongo_password
MONGO_HOST=your_mongo_host
MONGO_PORT=27017
MONGO_DB=your_database_name
```

## 🎯 Usage

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
- Verify RPC endpoint is accessible
- Check network connectivity
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

---

**Important Notes**:
- This tool requires a **local MongoDB instance** running on your machine with the SODAX backend database
- You cannot use this tool without having the database running locally
- Internet connectivity is required for blockchain RPC calls
- Make sure your environment is properly configured before use
