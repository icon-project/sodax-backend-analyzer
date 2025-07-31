# TODO: Validation Features Implementation

## Overview
This document outlines the validation features that need to be implemented for the sodax backend analyzer to validate token balances against on-chain data.

## Features to Implement

### 1. aToken Balance Validation for User
- **Flag**: `--validate-atoken-balance-of-user`
- **Parameter**: User address
- **Functionality**: 
  - Get the scaled aToken balance of a given user
  - Compare with real on-chain data
  - Fetch current related index for the specified token in the marketplace
  - Calculate real balance using the index
  - Fetch `balanceOf` from chain
  - Compare calculated vs on-chain balance

### 2. Variable Debt Token Balance Validation for User
- **Flag**: `--validate-debt-token-balance-of-user`
- **Parameter**: User address
- **Functionality**:
  - Same logic as aToken validation but for variable debt tokens
  - Get scaled debt token balance
  - Compare with on-chain data using appropriate index

### 3. Token Total Balance Validation (aToken)
- **Flag**: `--validate-atoken-balance`
- **Parameter**: aToken address
- **Functionality**:
  - Query reserve position with the aToken address
  - Fetch the index for calculation
  - Fetch supply on chain
  - Compare calculated total balance with on-chain supply

### 4. Token Total Balance Validation (Debt Token)
- **Flag**: `--validate-debt-token-balance`
- **Parameter**: Debt token address
- **Functionality**:
  - Same logic as aToken total balance validation but for debt tokens
  - Query reserve position with debt token address
  - Compare calculated vs on-chain total debt

### 5. Marketplace-wide Validation
- **Flag**: `--validate-all-tokens`
- **Functionality**:
  - Apply validation logic to all tokens in the marketplace
  - Output results showing:
    - Our calculated values
    - Real on-chain data
    - Comparison results
  - Define thresholds/conditions to determine if values are within accepted margin
  - Provide clear indicators for validation status

## Implementation Notes

### Required Components
- Index calculation logic for different token types
- On-chain data fetching mechanisms
- Balance calculation utilities
- Comparison and threshold validation logic
- Output formatting for validation results

### Considerations
- Define appropriate thresholds for acceptable margins
- Handle edge cases (zero balances, new tokens, etc.)
- Provide clear error messages for validation failures
- Consider performance implications for marketplace-wide validation
- Ensure proper error handling for network issues

### Output Format
Each validation should provide:
- Calculated balance/value
- On-chain balance/value
- Difference/percentage difference
- Validation status (PASS/FAIL)
- Threshold information used for validation 