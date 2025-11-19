# Balance Tracking for aToken and variableDebtToken via Events

## Table of Contents
- [Overview](#overview)
- [Key Concepts](#key-concepts)
- [Event Types and Calculations](#event-types-and-calculations)
  - [Mint Events](#mint-events)
  - [Burn Events](#burn-events)
  - [Transfer Events](#transfer-events)
- [Balance Retrieval](#balance-retrieval)
- [Implementation](#implementation)
- [Proof of Concept](#proof-of-concept)

## Overview

This document explains how to accurately track user balances for Aave V3 aTokens and variableDebtTokens by processing blockchain events. The key insight is that Aave stores balances in a "scaled" format internally, which must be converted to "real" balances using the current liquidity index.

## Key Concepts

### Scaled vs Real Balances
- **Scaled Balance**: The internal representation stored by Aave contracts
- **Real Balance**: The actual token amount that can be withdrawn/used
- **Liquidity Index**: A multiplier that converts between scaled and real balances
- **RAY**: A constant (`10^27`) used for precision in calculations

### Conversion Formula
```typescript
// Convert scaled balance to real balance
realBalance = scaledBalance * liquidityIndex / RAY

// Convert real balance to scaled balance  
scaledBalance = realBalance * RAY / liquidityIndex
```

## Event Types and Calculations

### Mint Events

When users deposit assets or receive interest, a `Mint` event is emitted.

#### Contract Flow
1. `AToken.mint()` is called
2. `ScaledBalanceTokenBase._mintScaled()` calculates the scaled amount
3. `MintableIncentivizedERC20._mint()` updates the internal balance
4. A `Mint` event is emitted

#### Key Contract Functions

**AToken.mint()**
```solidity
function mint(
    address caller,
    address onBehalfOf,
    uint256 amount,
    uint256 index
) external virtual override onlyPool returns (bool) {
    return _mintScaled(caller, onBehalfOf, amount, index);
}
```

**ScaledBalanceTokenBase._mintScaled()**
```solidity
function _mintScaled(
    address caller,
    address onBehalfOf,
    uint256 amount,
    uint256 index
) internal returns (bool) {
    uint256 amountScaled = amount.rayDiv(index);
    require(amountScaled != 0, Errors.INVALID_MINT_AMOUNT);

    uint256 scaledBalance = super.balanceOf(onBehalfOf);
    uint256 balanceIncrease = scaledBalance.rayMul(index) -
        scaledBalance.rayMul(_userState[onBehalfOf].additionalData);

    _userState[onBehalfOf].additionalData = index.toUint128();
    _mint(onBehalfOf, amountScaled.toUint128());

    uint256 amountToMint = amount + balanceIncrease;
    emit Transfer(address(0), onBehalfOf, amountToMint);
    emit Mint(caller, onBehalfOf, amountToMint, balanceIncrease, index);

    return (scaledBalance == 0);
}
```

#### Mint Event Structure
```solidity
event Mint(
    address indexed caller,
    address indexed onBehalfOf,
    uint256 amountToMint,
    uint256 balanceIncrease,
    uint256 index
);
```

#### Calculation Formula
```typescript
// Calculate scaled balance from Mint event
scaledBalance = (amountToMint - balanceIncrease) * RAY / index
```

### Burn Events

When users withdraw assets or repay debt, a `Burn` event is emitted.

#### Contract Flow
Similar to mint events, but the `_burnScaled()` function handles the logic:

```solidity
function _burnScaled(address user, address target, uint256 amount, uint256 index) internal {
    uint256 amountScaled = amount.rayDiv(index);
    require(amountScaled != 0, Errors.INVALID_BURN_AMOUNT);

    uint256 scaledBalance = super.balanceOf(user);
    uint256 balanceIncrease = scaledBalance.rayMul(index) -
        scaledBalance.rayMul(_userState[user].additionalData);

    _userState[user].additionalData = index.toUint128();
    _burn(user, amountScaled.toUint128());

    if (balanceIncrease > amount) {
        uint256 amountToMint = balanceIncrease - amount;
        emit Transfer(address(0), user, amountToMint);
        emit Mint(user, user, amountToMint, balanceIncrease, index);
    } else {
        uint256 amountToBurn = amount - balanceIncrease;
        emit Transfer(user, address(0), amountToBurn);
        emit Burn(user, target, amountToBurn, balanceIncrease, index);
    }
}
```

#### Calculation Formula
```typescript
// Calculate scaled balance from Burn event
scaledBalance = (balanceIncrease + amountToBurn) * RAY / index
```

### Transfer Events

aTokens can be transferred between users (variableDebtTokens are not transferable).

#### Contract Flow
1. `AToken._transfer()` gets the current liquidity index
2. `ScaledBalanceTokenBase._transfer()` handles the scaled transfer
3. Interest is accrued for both sender and recipient
4. A `Transfer` event is emitted with the real amount

#### Key Contract Functions

**AToken._transfer()**
```solidity
function _transfer(address from, address to, uint256 amount, bool validate) internal virtual {
    address underlyingAsset = _underlyingAsset;
    uint256 index = POOL.getReserveNormalizedIncome(underlyingAsset);

    uint256 fromBalanceBefore = super.balanceOf(from).rayMul(index);
    uint256 toBalanceBefore = super.balanceOf(to).rayMul(index);

    super._transfer(from, to, amount, index);

    if (validate) {
        POOL.finalizeTransfer(underlyingAsset, from, to, amount, fromBalanceBefore, toBalanceBefore);
    }

    emit BalanceTransfer(from, to, amount.rayDiv(index), index);
}
```

**ScaledBalanceTokenBase._transfer()**
```solidity
function _transfer(address sender, address recipient, uint256 amount, uint256 index) internal {
    // Calculate interest for sender and recipient
    uint256 senderScaledBalance = super.balanceOf(sender);
    uint256 senderBalanceIncrease = senderScaledBalance.rayMul(index) -
        senderScaledBalance.rayMul(_userState[sender].additionalData);

    uint256 recipientScaledBalance = super.balanceOf(recipient);
    uint256 recipientBalanceIncrease = recipientScaledBalance.rayMul(index) -
        recipientScaledBalance.rayMul(_userState[recipient].additionalData);

    _userState[sender].additionalData = index.toUint128();
    _userState[recipient].additionalData = index.toUint128();

    // Transfer the scaled amount
    super._transfer(sender, recipient, amount.rayDiv(index).toUint128());

    // Emit interest events
    if (senderBalanceIncrease > 0) {
        emit Transfer(address(0), sender, senderBalanceIncrease);
        emit Mint(_msgSender(), sender, senderBalanceIncrease, senderBalanceIncrease, index);
    }

    if (sender != recipient && recipientBalanceIncrease > 0) {
        emit Transfer(address(0), recipient, recipientBalanceIncrease);
        emit Mint(_msgSender(), recipient, recipientBalanceIncrease, recipientBalanceIncrease, index);
    }

    // Emit the actual transfer
    emit Transfer(sender, recipient, amount);
}
```

#### Important Considerations

1. **Index Retrieval**: Transfer events don't include the index. You need to:
   - Retrieve it from the contract at the time of the event, or
   - Use the index from `ReserveDataUpdated` events

2. **Zero Address Transfers**: Skip transfer events involving `0x000...000` address:
   - These are accompanied by `Mint` or `Burn` events
   - The tracking logic already handles these events
   - Including them would double-count the balance changes

#### Calculation Formula
```typescript
// Calculate scaled balance from Transfer event
scaledBalance = amount * RAY / index
```

## Balance Retrieval

To get a user's current balance, the contract uses this flow:

**AToken.balanceOf()**
```solidity
function balanceOf(address user) public view virtual override(IncentivizedERC20, IERC20) returns (uint256) {
    return super.balanceOf(user).rayMul(POOL.getReserveNormalizedIncome(_underlyingAsset));
}
```

**IncentivizedERC20.balanceOf()**
```solidity
function balanceOf(address account) public view virtual override returns (uint256) {
    return _userState[account].balance;
}
```

The contract:
1. Retrieves the scaled balance from `_userState[account].balance`
2. Multiplies it by the current liquidity index
3. Returns the real balance

## Implementation

### Tracking Strategy
1. **Process Events Chronologically**: Events must be processed in block order
2. **Maintain Scaled Balance**: Keep a running total of scaled balances
3. **Convert to Real Balance**: Use current index to convert when needed

### Event Processing Rules
- **Mint Events**: Add scaled amount to balance
- **Burn Events**: Subtract scaled amount from balance  
- **Transfer Events**: 
  - Add scaled amount if recipient is target user
  - Subtract scaled amount if sender is target user
  - Skip if involves zero address

### Balance Calculation
```typescript
// Final real balance calculation
realBalance = trackedScaledBalance * currentIndex / RAY
```

## Proof of Concept

A proof-of-concept script has been created to verify this logic:

### Files
- **`balance-tracking-poc.js`**: Main script implementing the balance tracking logic
- **`data*.json`**: Real event data from on-chain sources for testing

### Key Features
- Tracks both scaled and real balances simultaneously
- Compares calculated balances with on-chain values
- Provides detailed logging of each event's impact
- Validates accuracy within tolerance thresholds

### Usage
```bash
# Run with default data file
node balance-tracking-poc.js

# Run with specific data file
node balance-tracking-poc.js data2.json
```

### Validation Results
The POC script validates that:
- Event-based tracking matches on-chain balances
- Both calculation methods (scaled vs real) produce accurate results
- The logic correctly handles all event types and edge cases

**Important Note**: Direct real balance tracking currently appears accurate due to our limited dataset size and minimal interest accrual. With small datasets, the liquidity index remains very close to 1 (the initial value), making the difference between scaled and real balance tracking negligible. In production environments with longer time periods and significant interest accrual, the scaled balance tracking method will be more accurate as it properly accounts for index changes over time.

---

## References

### Aave Protocol Smart Contracts
- **[Aave V3 Core Repository](https://github.com/aave/aave-v3-core)**: Main smart contract implementation
- **[AToken Contract](https://github.com/aave/aave-v3-core/blob/master/contracts/protocol/tokenization/AToken.sol)**: Interest-bearing token implementation
- **[ScaledBalanceTokenBase](https://github.com/aave/aave-v3-core/blob/master/contracts/protocol/tokenization/base/ScaledBalanceTokenBase.sol)**: Base contract for scaled balance tokens
- **[MintableIncentivizedERC20](https://github.com/aave/aave-v3-core/blob/master/contracts/protocol/tokenization/base/MintableIncentivizedERC20.sol)**: Base contract for mintable incentivized tokens

### Protocol Subgraphs
- **[Aave Protocol Subgraphs](https://github.com/aave/protocol-subgraphs)**: Official subgraph implementations
- **[Tokenization V3 Mapping](https://github.com/aave/protocol-subgraphs/blob/a00e51fd16aa0defccb796fe395ea6fe83478991/src/mapping/tokenization/tokenization-v3.ts)**: Event processing logic for V3 tokens

**Note**: This implementation follows the same logic used in the [Aave protocol subgraphs](https://github.com/aave/protocol-subgraphs/blob/a00e51fd16aa0defccb796fe395ea6fe83478991/src/mapping/tokenization/tokenization-v3.ts#L226C5-L228C94).
