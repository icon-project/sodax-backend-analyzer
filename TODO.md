# TODO: SODAX Backend Analyzer

## 🆕 New CLI Features

### Data Fetching Commands
- [x] **--get-all-users**: Print all user addresses from the database
- [x] **--get-all-reserves**: Print reserve token addresses and symbols
- [x] **--get-all-a-token**: Print aToken addresses and symbols
- [x] **--get-all-debt-token**: Print debt token addresses and symbols

### Event Retrieval Commands
- [x] **--get-token-events**: Get events for a specific token (accepts token address directly)
- [x] **--get-user-events**: Get events for a specific user (accepts user address directly)

### Index Validation Commands
- [x] **--validate-reserve-indexes**: Validate liquidity and borrow indexes for a specific reserve
- [x] **--validate-all-reserve-indexes**: Validate indexes for all reserves

### Code Refactoring
- [x] **Rename VariableToken to DebtToken**: Update all references from VariableToken to DebtToken
- [x] **Rename --variable-token to --debt-token**: Update CLI flag from --variable-token to --debt-token

### Remote Database Connectivity
- [ ] **Development Environment Support**: Add ability to connect to development MongoDB instance via remote URI
  - [ ] Add configuration option for remote MongoDB URI
  - [ ] Update environment variables to support remote connections
  - [ ] Add connection string validation for remote URIs
  - [ ] Update documentation for remote database setup
- [ ] **Production Environment Support**: Add ability to connect to production MongoDB instance
  - [ ] Add secure connection handling for production databases
  - [ ] Implement connection pooling for production workloads
  - [ ] Add authentication and authorization for production access
  - [ ] Add connection retry logic and failover handling
- [ ] **Environment Selection**: Add CLI flags to select between local, development, and production environments
  - [ ] `--env local` (default) - Use local MongoDB instance
  - [ ] `--env dev` - Connect to development MongoDB instance
  - [ ] `--env prod` - Connect to production MongoDB instance
- [ ] **Configuration Management**: Enhance configuration system for multiple environments
  - [ ] Support for environment-specific `.env` files
  - [ ] Configuration validation for each environment
  - [ ] Fallback configuration handling

### SODAX Backend API Integration
- [ ] **HTTP/HTTPS Client Setup**: Implement HTTP client for SODAX backend API
  - [ ] Add HTTP client library (reqwest or similar)
  - [ ] Implement base API client with authentication
  - [ ] Add request/response models for API data
  - [ ] Add error handling for API requests
- [ ] **Orderbook API Integration**: Connect to `/orderbook` endpoint
  - [ ] Implement pagination support (page, limit parameters)
  - [ ] Add CLI flags for orderbook pagination (`--page`, `--limit`)
  - [ ] Create orderbook data models matching API response
  - [ ] Add orderbook caching for performance
- [ ] **Money Market API Integration**: Prepare for upcoming money market endpoints
  - [ ] Design extensible API client architecture
  - [ ] Add configuration for money market API base URL
  - [ ] Create placeholder handlers for future endpoints
  - [ ] Add API versioning support
- [ ] **API vs Database Mode**: Add CLI flags to choose data source
  - [ ] `--source db` (default) - Use direct MongoDB connection
  - [ ] `--source api` - Use SODAX backend API
  - [ ] `--source hybrid` - Use API for some operations, DB for others
- [ ] **API Configuration**: Add environment variables for API endpoints
  - [ ] `SODAX_API_BASE_URL` - Base URL for SODAX backend API
  - [ ] `SODAX_API_KEY` - API authentication key (if required)
  - [ ] `SODAX_API_TIMEOUT` - Request timeout configuration
  - [ ] `SODAX_API_ENV` - Environment selection (local, dev, prod)
  - [ ] `SODAX_API_LOCAL_URL` - Local API endpoint (e.g., http://localhost:3000)
  - [ ] `SODAX_API_DEV_URL` - Development API endpoint
  - [ ] `SODAX_API_PROD_URL` - Production API endpoint
  - [ ] Auto-select API URL based on `SODAX_API_ENV` environment variable
- [ ] **API Response Handling**: Implement robust API response processing
  - [ ] Add response validation and error handling
  - [ ] Implement retry logic for failed requests
  - [ ] Add rate limiting support
  - [ ] Add response caching for frequently accessed data

## 🧪 Testing

### Update Tests for New Features
- [ ] **Update integration tests** to cover new validation features
  - [ ] Test individual validation commands (`--validate-user-supply`, `--validate-user-borrow`, etc.)
  - [ ] Test bulk validation commands (`--validate-user-all`, `--validate-users-all`, `--validate-token-all`, `--validate-all`)
  - [ ] Test error handling scenarios (rate limits, missing data, etc.)
  - [ ] Test the new data structures (`UserPositionValidation`, `UserEntryState`, `ReserveEntryState`)
  - [ ] Test position-level error tracking
  - [ ] Test graceful degradation when individual validations fail

## 🚀 Performance & UX Improvements

### Real-time Console Updates
- [ ] **Implement progress indicators** for bulk validation operations
  - [ ] Show progress bar or percentage for large operations
  - [ ] Print validation results as they complete (not just at the end)
  - [ ] Add timing information for each validation
  - [ ] Show current item being processed (e.g., "Validating user 45/100...")
  - [ ] Consider using async streams or channels for real-time updates

### Validation Summary Function
- [ ] **Create comprehensive summary function** that analyzes validation results
  - [ ] Define validation conditions/thresholds (e.g., acceptable percentage differences)
  - [ ] Count passed vs failed validations based on conditions
  - [ ] Print only failed validations with details
  - [ ] Show summary statistics (total, passed, failed, success rate)
  - [ ] Support different summary formats (brief, detailed, error-only)
  - [ ] Add configurable thresholds via CLI flags or config file

## 🔧 Technical Improvements

### Code Quality
- [ ] **Add comprehensive error handling** for edge cases
- [ ] **Improve logging** for debugging and monitoring
- [ ] **Add performance benchmarks** for bulk operations

### Configuration
- [ ] **Add configuration system** for validation thresholds
- [ ] **Add retry logic** for failed RPC calls
- [ ] **Add rate limiting configuration** for RPC endpoints

## 📊 Future Enhancements

### Output Formats
- [ ] **Add JSON output** for programmatic use
- [ ] **Add CSV export** for data analysis
- [ ] **Add structured logging** for monitoring

### Advanced Features
- [ ] **Add validation history** tracking
- [ ] **Add caching** for frequently accessed data
- [ ] **Add validation scheduling** (periodic validation)

## 🎯 Priority Order

1. **High Priority**: Update tests for new features
2. **High Priority**: Real-time console updates for bulk operations
3. **High Priority**: Validation summary function
4. **Medium Priority**: Configuration system
5. **Medium Priority**: Scaled balance rolling calculations
6. **Low Priority**: Advanced features and output formats

## 🌟 Nice to Have - Future Enhancements

### Error Handling & Observability
- [ ] Replace all `unwrap!/expect!` with proper error handling using `anyhow` + `thiserror`
- [ ] Make `main()` return `Result<()>` and bubble errors with context
- [ ] Add `color-eyre` for pretty local error reports
- [ ] Add `tracing` + `tracing-subscriber` with structured fields (`chain_id`, `reserve`, `wallet`, `op`)
- [ ] Support env-configurable log levels (`RUST_LOG`) and JSON summary output with `--format json`

### CLI & Configuration
- [ ] Migrate CLI to `clap` derive with subcommands (`validate reserves|users|positions`) and common flags
- [ ] Add a validated `Config` struct from env + CLI
- [ ] Add `--from-block`, `--to-block`, and `--snapshot <file>` options to make runs reproducible
- [ ] Implement `--dry-run` mode and clear exit codes

### Database & Performance
- [ ] Create a single `mongodb::Client` per process and pass handles down
- [ ] Replace `spawn + join_all` with bounded concurrency (`FuturesUnordered` + `.buffer_unordered` or `Semaphore`)
- [ ] Add exponential backoff with jitter for RPC/Mongo calls
- [ ] Add `metrics` + `metrics-exporter-prometheus` (or OTLP) with counters and histograms

### Testing & Quality
- [ ] Write unit tests for pure functions; gate network/DB tests behind `integration` feature
- [ ] Replace placeholder asserts with real invariants
- [ ] Set up GitHub Actions for `fmt --check`, `clippy -D warnings`, and tests
- [ ] Implement property tests with `proptest` for math and formatting
- [ ] Add regression seeds to tests
- [ ] Build integration harness using `testcontainers` for Mongo and a mock RPC client
- [ ] Add criterion benchmarks for hot paths; document optimal concurrency

### Security & Reliability
- [ ] Mask secrets in logs; validate inputs; add timeouts and circuit breakers
- [ ] Add MIT or Apache-2.0 LICENSE and CI badge

### Distribution & Deployment
- [ ] Package with `cargo-dist` or release workflow for Linux/macOS/Windows
- [ ] Provide `docker-compose` sandbox with Mongo + mock RPC and seed data
- [ ] Add `make dev` and `make test` targets

### Documentation & Examples
- [ ] Expand docs with architecture diagram, failure modes, observability guide, and "Why Rust here?" section
- [ ] Create Loom/GIF demo
- [ ] Add 2–3 showcase scenarios with sample outputs (human & JSON) and small dataset

### Advanced Features (Stretch Goals)
- [ ] (Stretch) Swap critical math to `U256` from `alloy` and centralize helpers
- [ ] (Stretch) Add OpenTelemetry exporter for traces
- [ ] (Stretch) Feature-gate alternative parallelism strategies (rayon vs tokio)
