# TODO: SODAX Backend Analyzer

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

## 📊 Data Analysis & Calculations

### Scaled Balance Rolling Calculations
- [ ] **Calculate scaled balance rolling from money_market_event collection**
  - [ ] For a given reserve_token, calculate rolling scaled balance
  - [ ] Add balance on every mint event
  - [ ] Decrease balance on every burn event
  - [ ] Maintain running total in chronological order
  - [ ] Handle edge cases (missing events, invalid data)
- [ ] **Calculate scaled balance rolling for user positions**
  - [ ] For a given user_position, calculate rolling scaled balance
  - [ ] Add balance on every mint event for that user
  - [ ] Decrease balance on every burn event for that user
  - [ ] Maintain running total in chronological order
  - [ ] Handle edge cases (missing events, invalid data)

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