const fs = require('fs');
const path = require('path');

// ============================================================================
// CONFIGURATION
// ============================================================================

// Define the sample data file to use (can be passed as command line argument)
const SAMPLE_DATA_FILE = process.argv[2] || 'data.json';

// Aave V3 protocol constants
const RAY = 10n ** 27n; // Used for precision in calculations
const FALLBACK_BASELINE = 10_000_000n; // 10 million wei for safe percentage calculations

// ============================================================================
// DATA LOADING
// ============================================================================

// Load event data from external JSON file
const loadEventData = () => {
  const dataPath = path.join(__dirname, SAMPLE_DATA_FILE);
  const rawData = fs.readFileSync(dataPath, 'utf8');
  return JSON.parse(rawData);
};

const eventData = loadEventData();
const events = eventData.events;
const currentIndex = BigInt(eventData.lastIndex);
const onChainBalance = BigInt(eventData.onChainBalance);
const userAddress = eventData.user;

// ============================================================================
// CORE CALCULATION FUNCTIONS
// ============================================================================

/**
 * Calculates the scaled balance from event parameters
 * @param {string|number} value - The event value (amount)
 * @param {string|number} index - The liquidity index at the time of the event
 * @param {string|number|null} balanceIncrease - Interest accrued (null for transfers)
 * @param {string} eventType - Type of event: 'mint', 'burn', or 'transfer'
 * @param {bigint} lastKnownIndex - Last known index for transfer events
 * @returns {bigint} The scaled balance
 */
const calculateScaledBalance = (value, index, balanceIncrease = null, eventType = 'mint', lastKnownIndex = RAY) => {
  const bigValue = BigInt(value);
  const bigIndex = eventType === 'transfer' ? lastKnownIndex : BigInt(index);
  const bigBalanceIncrease = balanceIncrease ? BigInt(balanceIncrease) : 0n;

  if (eventType === 'mint') {
    return ((bigValue - bigBalanceIncrease) * RAY) / bigIndex;
  } else if (eventType === 'burn') {
    return ((bigValue + bigBalanceIncrease) * RAY) / bigIndex;
  } else if (eventType === 'transfer') {
    return (bigValue * RAY) / bigIndex;
  }

  throw new Error(`Unsupported event type: ${eventType}`);
};

/**
 * Converts scaled balance back to real balance using current index
 * @param {bigint} scaledBalance - The scaled balance
 * @param {bigint} index - The current liquidity index
 * @returns {bigint} The real balance
 */
const convertScaledToRealBalance = (scaledBalance, index) => {
  const bigScaled = BigInt(scaledBalance);
  const bigIndex = BigInt(index);
  return (bigScaled * bigIndex) / RAY;
};

/**
 * Determines if a transfer event should be skipped (involves zero address)
 * @param {Object} event - The event object
 * @returns {boolean} True if event should be skipped
 */
const shouldSkipTransferEvent = event => {
  if (!event.eventType.includes('transfer')) return false;

  const zeroAddress = '0x0000000000000000000000000000000000000000';
  return event.to === zeroAddress || event.from === zeroAddress;
};

/**
 * Determines the event type for calculation purposes
 * @param {string} eventType - The full event type string
 * @returns {string} Simplified event type: 'mint', 'burn', or 'transfer'
 */
const getEventTypeForCalculation = eventType => {
  if (eventType.includes('mint')) return 'mint';
  if (eventType.includes('burn')) return 'burn';
  if (eventType.includes('transfer')) return 'transfer';
  throw new Error(`Unhandled event type: ${eventType}`);
};

// ============================================================================
// MAIN BALANCE TRACKING LOGIC
// ============================================================================

/**
 * Processes all events and tracks both scaled and real balances
 * @returns {Object} Object containing final scaled and real balances
 */
const processEvents = () => {
  console.log('CALCULATING BOTH METHODS SIMULTANEOUSLY');
  console.log('=======================================\n');

  let scaledBalance = 0n;
  let realBalance = 0n;
  let lastIndex = RAY;

  events.forEach((event, index) => {
    // Skip transfer events involving zero address (handled by Mint/Burn events)
    if (shouldSkipTransferEvent(event)) {
      console.log(`Skipping transfer event: ${event.eventType} at block ${event.blockNumber}`);
      return;
    }

    // Update last known index for mint events
    if (event.eventType.includes('a-token-mint')) {
      lastIndex = BigInt(event.index);
    }

    const eventType = getEventTypeForCalculation(event.eventType);
    const eventScaled = calculateScaledBalance(event.value, event.index, event.balanceIncrease, eventType, lastIndex);

    const realAmount = BigInt(event.value); // Already rebased at time of event

    const scaledBefore = scaledBalance;
    const realBefore = realBalance;

    // Update balances based on event type and direction
    if (event.eventType === 'debt-token-mint' || event.eventType === 'a-token-mint') {
      scaledBalance += eventScaled;
      realBalance += realAmount;
    } else if (event.eventType === 'debt-token-burn' || event.eventType === 'a-token-burn') {
      scaledBalance -= eventScaled;
      realBalance -= realAmount;
    } else if (event.eventType === 'a-token-transfer') {
      if (event.to === userAddress) {
        scaledBalance += eventScaled;
        realBalance += realAmount;
      } else if (event.from === userAddress) {
        scaledBalance -= eventScaled;
        realBalance -= realAmount;
      }
    } else {
      throw new Error(`Unhandled event type: ${event.eventType}`);
    }

    // Log event processing details
    logEventProcessing(
      index,
      event,
      eventType,
      eventScaled,
      scaledBefore,
      scaledBalance,
      realBefore,
      realBalance,
      lastIndex,
    );
  });

  return { scaledBalance, realBalance };
};

/**
 * Logs detailed information about event processing
 */
const logEventProcessing = (
  index,
  event,
  eventType,
  eventScaled,
  scaledBefore,
  scaledBalance,
  realBefore,
  realBalance,
  lastIndex,
) => {
  console.log(`${(index + 1).toString().padStart(3, '0')}. ${event.eventType}`);
  console.log(`   Block:  ${event.blockNumber.padStart(28, ' ')}`);
  console.log(`   Value:  ${event.value.padStart(28, ' ')}`);
  console.log(`   Balance Increase:  ${event.balanceIncrease.toString().padStart(17, ' ')}`);

  const displayIndex = eventType === 'transfer' ? lastIndex : event.index;
  console.log(`   Index:  ${displayIndex.toString().padStart(20, ' ')}`);
  console.log(`   Scaled: ${eventScaled.toString().padStart(28, ' ')}`);
  console.log(
    `   Scaled Balance: ${scaledBefore.toString().padStart(20, ' ')} → ${scaledBalance.toString().padStart(20, ' ')}`,
  );
  console.log(
    `   Real Balance:   ${realBefore.toString().padStart(20, ' ')} → ${realBalance.toString().padStart(20, ' ')}`,
  );
  console.log('');
};

// ============================================================================
// RESULTS COMPARISON AND ANALYSIS
// ============================================================================

/**
 * Compares calculated balances with on-chain balance and provides analysis
 * @param {bigint} finalScaledBalance - Final balance calculated from scaled events
 * @param {bigint} finalRealBalance - Final balance calculated from real events
 */
const compareWithOnChainBalance = (finalScaledBalance, finalRealBalance) => {
  const finalFromScaled = convertScaledToRealBalance(finalScaledBalance, currentIndex);

  console.log(`Final balance (scaled per event): ${finalFromScaled.toString().padStart(10, ' ')}`);
  console.log(`Final real balance (simple sum): ${finalRealBalance.toString().padStart(10, ' ')}`);

  console.log('\n========================================');
  console.log('ON-CHAIN COMPARISON:');
  console.log('========================================');

  console.log(`On-chain balance:        ${onChainBalance.toString().padStart(28, ' ')}`);
  console.log(`Current index:           ${currentIndex.toString().padStart(20, ' ')}`);
  console.log(`Method 1 (scaled per event): ${finalFromScaled.toString().padStart(24, ' ')}`);
  console.log(`Method 2 (sum real):    ${finalRealBalance.toString().padStart(24, ' ')}`);

  // Calculate differences
  const diff1 = onChainBalance > finalFromScaled ? onChainBalance - finalFromScaled : finalFromScaled - onChainBalance;
  const diff2 = realBalance > onChainBalance ? realBalance - onChainBalance : onChainBalance - realBalance;

  // Choose safe denominator for percent calculation
  const safeDenominator = onChainBalance === 0n ? FALLBACK_BASELINE : onChainBalance;

  // Calculate percentage differences with 4 decimal precision
  const percentDiff1 = Number((diff1 * 1000000n) / safeDenominator) / 10000;
  const percentDiff2 = Number((diff2 * 1000000n) / safeDenominator) / 10000;

  // Log results
  console.log(`\nMethod 1 difference:     ${diff1.toString().padStart(20, ' ')} (${percentDiff1}%)`);
  console.log(`Method 2 difference:     ${diff2.toString().padStart(20, ' ')} (${percentDiff2}%)`);

  // Determine which method is better
  if (Math.abs(percentDiff1) < Math.abs(percentDiff2)) {
    console.log('\n✅ Method 1 (scale per event) is more accurate');
  } else if (Math.abs(percentDiff2) < Math.abs(percentDiff1)) {
    console.log('\n✅ Method 2 (scale at end) is more accurate');
  } else {
    console.log('\n⚠️  Both methods have similar accuracy');
  }

  // Overall assessment
  const bestDiff = Math.min(Math.abs(percentDiff1), Math.abs(percentDiff2));
  if (bestDiff < 0.01) {
    console.log('\n✅ Best method matches within 0.01% tolerance');
  } else if (bestDiff < 1) {
    console.log('\n⚠️  Best method differs by less than 1% - minor discrepancy');
  } else {
    console.log('\n❌ Significant balance discrepancy detected');
  }
};

// ============================================================================
// MAIN EXECUTION
// ============================================================================

// Run the balance tracking analysis
const { scaledBalance, realBalance } = processEvents();
compareWithOnChainBalance(scaledBalance, realBalance);
