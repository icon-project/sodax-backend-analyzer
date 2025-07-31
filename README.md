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

## 📋 Prerequisites

- **Rust** (latest stable version)
- **MongoDB** instance running (a copy of the database is necessary)
- **Environment variables** configured (see Configuration section)

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

# Get reserve token data by reserve address
cargo run -- --reserve-token <RESERVE_ADDRESS>

# Get reserve token data by aToken address
cargo run -- --a-token <ATOKEN_ADDRESS>

# Get reserve token data by variable debt token address
cargo run -- --variable-token <VARIABLE_TOKEN_ADDRESS>

# Get user position data by wallet address
cargo run -- --user-position <WALLET_ADDRESS>

# Get token balance for a user (requires token type flag)
cargo run -- --balance-of <USER_ADDRESS> --reserve-token <TOKEN_ADDRESS>
```

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

# Get user balance for a specific token
cargo run -- --balance-of 0xuser123... --reserve-token 0xtoken456...
```

## 🏗️ Project Structure

```
sodax-backend-analizer/
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library entry point and CLI logic
│   ├── config.rs            # Configuration management
│   ├── db.rs                # Database operations
│   ├── evm.rs               # EVM blockchain integration
│   └── models.rs            # Data models
├── tests/
│   └── mongodb_integration_tests.rs
├── Cargo.toml
├── Cargo.lock
├── Makefile
└── README.md
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

### Adding New Features

1. **Database Functions**: Add new functions in `src/db.rs`
2. **CLI Commands**: Update `src/lib.rs` with new flag handling
3. **EVM Functions**: Add blockchain interactions in `src/evm.rs`
4. **Data Models**: Add new models in `src/models.rs`
5. **Tests**: Add corresponding tests in `tests/`

## 📊 Data Models

### ReserveTokenDocument
```rust
pub struct ReserveTokenDocument {
    pub id: ObjectId,
    pub totalATokenBalance: Decimal128,
    pub totalVariableDebtTokenBalance: Decimal128,
    pub suppliers: Vec<String>,
    pub borrowers: Vec<String>,
    pub aTokenAddress: String,
    pub variableDebtTokenAddress: String,
    pub reserveAddress: String,
    pub symbol: String,
    pub liquidityRate: Decimal128,
    pub stableBorrowRate: Decimal128,
    pub variableBorrowRate: Decimal128,
    pub liquidityIndex: Decimal128,
    pub variableBorrowIndex: Decimal128,
    pub blockNumber: u64,
    pub createdAt: DateTime,
    pub updatedAt: DateTime,
    pub version: i32,
}
```

## 🔍 Data Sources

### MongoDB Collections
The tool connects to the following MongoDB collections:

- `reserve_tokens` - Reserve token data
- `user_positions` - User position data
- `orderbook` - Orderbook information
- `moneyMarketEvents` - Money market events
- `walletFactoryEvents` - Wallet factory events
- `intentEvents` - Intent events

### Blockchain Integration
The tool also connects to Ethereum-compatible blockchains:

- **RPC Endpoint**: `https://rpc.soniclabs.com`
- **Supported Operations**:
  - Get latest block number
  - Query ERC20 token balances
  - Contract interactions via Solidity bindings

## 🚨 Error Handling

The application handles various error scenarios:

- **Database connection errors** - Graceful error messages
- **Blockchain RPC errors** - Network and contract interaction errors
- **Missing environment variables** - Clear configuration instructions
- **Invalid CLI arguments** - Helpful usage information with validation rules
- **Data not found** - Appropriate "not found" messages
- **Address parsing errors** - Invalid Ethereum address format handling

## 🤝 Contributing

1. **Fork the repository**
2. **Create a feature branch**: `git checkout -b feature/new-feature`
3. **Make your changes**
4. **Run tests**: `cargo test`
5. **Check code quality**: `cargo clippy`
6. **Commit your changes**: `git commit -m "Add new feature"`
7. **Push to the branch**: `git push origin feature/new-feature`
8. **Submit a pull request**

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

## 📞 Support

For issues and questions:
- Create an issue in the repository
- Check existing issues for solutions
- Review the documentation

---

**Note**: This tool requires a running MongoDB instance with the appropriate SODAX backend data and internet connectivity for blockchain RPC calls. Make sure your environment is properly configured before use. 