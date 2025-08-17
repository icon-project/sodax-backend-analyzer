use mongodb::bson::{oid::ObjectId, DateTime, Decimal128};
use serde::{Deserialize, Serialize};

mod serde_helpers {
    use mongodb::bson::Decimal128;
    use serde::{Deserialize, Deserializer};

    #[allow(dead_code)]
    pub(super) fn decimal128_to_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let d = Decimal128::deserialize(deserializer)?;
        d.to_string()
            .parse::<u64>()
            .map_err(serde::de::Error::custom)
    }
}

pub enum CollectionTypes {
    OrderbookDocument,
    ReserveTokenDocument,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(non_snake_case)]
pub struct UserAssetPositionDocument {
    pub reserveAddress: String,
    pub aTokenAddress: String,
    pub variableDebtTokenAddress: String,
    pub aTokenBalance: Decimal128,
    pub variableDebtTokenBalance: Decimal128,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct UserPositionDocument {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub userAddress: String,
    pub positions: Vec<UserAssetPositionDocument>,
    pub createdAt: DateTime,
    pub updatedAt: DateTime,
    #[serde(rename = "__v")]
    pub version: i32,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct ReserveTokenDocument {
    #[serde(rename = "_id")]
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
    #[serde(rename = "__v")]
    pub version: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(non_snake_case)]
pub struct OrderbookDocument {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub intentState: IntentState,
    pub intentData: IntentData,
    pub createdAt: DateTime,
    pub updatedAt: DateTime,
    #[serde(rename = "__v")]
    pub version: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(non_snake_case)]
pub struct IntentState {
    pub exists: bool,
    pub remainingInput: Decimal128,
    pub receivedOutput: Decimal128,
    pub pendingPayment: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(non_snake_case)]
pub struct IntentData {
    pub intentId: String,
    pub creator: String,
    pub txHash: String,
    pub inputToken: String,
    pub outputToken: String,
    pub inputAmount: Decimal128,
    pub minOutputAmount: Decimal128,
    pub deadline: Decimal128,
    pub allowPartialFill: bool,
    pub srcChain: Decimal128,
    pub dstChain: Decimal128,
    pub srcAddress: String,
    pub dstAddress: String,
    pub solver: String,
    pub data: String,
    pub intentHash: String,
    pub blockNumber: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(non_snake_case)]
pub struct SolverVolumeDocument {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub txHash: String,
    pub intentHash: String,
    pub solver: String,
    pub outputToken: String,
    pub amount: Decimal128,
    pub chainId: u64,
    pub blockNumber: u64,
    pub timestamp: Option<DateTime>,
    data: String,
    #[serde(rename = "__v")]
    pub version: i32,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct SolverVolumeTimestampAndBlock {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub blockNumber: u64,
    pub timestamp: Option<DateTime>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(non_snake_case)]
pub struct CommonFields {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub txHash: String,
    pub logIndex: i64,
    pub chainId: u64,
    pub blockNumber: u64,
    #[serde(rename = "__v")]
    pub version: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(non_snake_case)]
pub struct IntentCreatedEvent {
    #[serde(flatten)]
    pub common: CommonFields,
    pub intent: IntentData,
    pub intentHash: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(non_snake_case)]
pub struct IntentFilledEvent {
    #[serde(flatten)]
    pub common: CommonFields,
    pub solver: String,
    pub inputToken: String,
    pub outputToken: String,
    pub inputAmount: Decimal128,
    pub outputAmount: Decimal128,
    pub recipient: String,
    pub intentHash: String,
}

// #[derive(Debug, Deserialize, Serialize, Clone)]
// #[allow(non_snake_case)]
// pub struct IntentExternalFillFailedEvent {
//     #[serde(flatten)]
//     pub common: CommonFields,
//     fillId: Decimal128,
//     tsHash: String,
//     logIndex: i64,
//     chainId: u64,
//     blockNumber: u64,
// }

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(non_snake_case)]
pub struct IntentCancelledEvent {
    #[serde(flatten)]
    pub common: CommonFields,
    pub intentHash: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(non_snake_case)]
#[allow(clippy::large_enum_variant)]
#[serde(tag = "eventType")]
pub enum IntentEventDocument {
    #[serde(rename = "intent-created")]
    IntentCreated(IntentCreatedEvent),
    #[serde(rename = "intent-filled")]
    IntentFilled(IntentFilledEvent),
    // #[serde(rename = "intent-external-fill-failed")]
    // IntentExternalFillFailed(IntentExternalFillFailedEvent),
    #[serde(rename = "intent-cancelled")]
    IntentCancelled(IntentCancelledEvent),
}

impl IntentEventDocument {
    fn common(&self) -> &CommonFields {
        match self {
            IntentEventDocument::IntentCreated(e) => &e.common,
            IntentEventDocument::IntentFilled(e) => &e.common,
            // IntentEventDocument::IntentExternalFillFailed(e) => &e.common,
            IntentEventDocument::IntentCancelled(e) => &e.common,
        }
    }
    pub fn id(&self) -> ObjectId {
        self.common().id
    }

    pub fn tx_hash(&self) -> &str {
        &self.common().txHash
    }
    pub fn block_number(&self) -> u64 {
        self.common().blockNumber
    }

    pub fn chain_id(&self) -> u64 {
        self.common().chainId
    }
    pub fn log_index(&self) -> i64 {
        self.common().logIndex
    }

    pub fn version(&self) -> i32 {
        self.common().version
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            IntentEventDocument::IntentCreated(_) => "intent-created",
            IntentEventDocument::IntentFilled(_) => "intent-filled",
            // IntentEventDocument::IntentExternalFillFailed(_) => "intent-external-fill-failed",
            IntentEventDocument::IntentCancelled(_) => "intent-cancelled",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ATokenBalanceTransferEvent {
    #[serde(flatten)]
    pub common: CommonFields,
    pub tokenAddress: String,
    pub from: String,
    pub to: String,
    pub value: Decimal128,
    pub index: Decimal128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ATokenBurnEvent {
    #[serde(flatten)]
    pub common: CommonFields,
    pub tokenAddress: String,
    pub from: String,
    pub target: String,
    pub value: Decimal128,
    pub balanceIncrease: Decimal128,
    pub index: Decimal128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ATokenMintEvent {
    #[serde(flatten)]
    pub common: CommonFields,
    pub tokenAddress: String,
    pub caller: String,
    pub onBehalfOf: String,
    pub value: Decimal128,
    pub balanceIncrease: Decimal128,
    pub index: Decimal128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ATokenTransferEvent {
    #[serde(flatten)]
    pub common: CommonFields,
    pub tokenAddress: String,
    pub from: String,
    pub to: String,
    pub value: Decimal128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct DebtTokenMintEvent {
    #[serde(flatten)]
    pub common: CommonFields,
    pub tokenAddress: String,
    pub caller: String,
    pub onBehalfOf: String,
    pub value: Decimal128,
    pub balanceIncrease: Decimal128,
    pub index: Decimal128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct DebtTokenBurnEvent {
    #[serde(flatten)]
    pub common: CommonFields,
    pub tokenAddress: String,
    pub from: String,
    pub target: String,
    pub value: Decimal128,
    pub balanceIncrease: Decimal128,
    pub index: Decimal128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ReserveDataUpdatedEvent {
    #[serde(flatten)]
    pub common: CommonFields,
    pub reserve: String,
    pub liquidityRate: Decimal128,
    pub stableBorrowRate: Decimal128,
    pub variableBorrowRate: Decimal128,
    pub liquidityIndex: Decimal128,
    pub variableBorrowIndex: Decimal128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct SupplyEvent {
    #[serde(flatten)]
    pub common: CommonFields,
    pub reserve: String,
    pub user: String,
    pub onBehalfOf: String,
    pub amount: Decimal128,
    pub referralCode: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct BorrowEvent {
    #[serde(flatten)]
    pub common: CommonFields,
    pub reserve: String,
    pub user: String,
    pub onBehalfOf: String,
    pub amount: Decimal128,
    pub interestRateMode: i32,
    pub borrowRate: Decimal128,
    pub referralCode: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct RepayEvent {
    #[serde(flatten)]
    pub common: CommonFields,
    pub reserve: String,
    pub user: String,
    pub repayer: String,
    pub amount: Decimal128,
    pub useATokens: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct WithdrawEvent {
    #[serde(flatten)]
    pub common: CommonFields,
    pub reserve: String,
    pub user: String,
    pub to: String,
    pub amount: Decimal128,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(non_snake_case)]
#[allow(clippy::large_enum_variant)]
#[serde(tag = "eventType")]
pub enum MoneyMarketEventDocument {
    #[serde(rename = "a-token-balance-transfer")]
    ATokenBalanceTransfer(ATokenBalanceTransferEvent),

    #[serde(rename = "a-token-burn")]
    ATokenBurn(ATokenBurnEvent),

    #[serde(rename = "a-token-mint")]
    ATokenMint(ATokenMintEvent),

    #[serde(rename = "a-token-transfer")]
    ATokenTransfer(ATokenTransferEvent),

    #[serde(rename = "borrow")]
    Borrow(BorrowEvent),

    #[serde(rename = "debt-token-burn")]
    DebtTokenBurn(DebtTokenBurnEvent),

    #[serde(rename = "debt-token-mint")]
    DebtTokenMint(DebtTokenMintEvent),

    #[serde(rename = "repay")]
    Repay(RepayEvent),

    #[serde(rename = "reserve-data-updated")]
    ReserveDataUpdated(ReserveDataUpdatedEvent),

    #[serde(rename = "supply")]
    Supply(SupplyEvent),

    #[serde(rename = "withdraw")]
    Withdraw(WithdrawEvent),
}

impl MoneyMarketEventDocument {
    fn common(&self) -> &CommonFields {
        match self {
            Self::ATokenBalanceTransfer(e) => &e.common,
            Self::ATokenBurn(e) => &e.common,
            Self::ATokenMint(e) => &e.common,
            Self::ATokenTransfer(e) => &e.common,
            Self::Borrow(e) => &e.common,
            Self::DebtTokenBurn(e) => &e.common,
            Self::DebtTokenMint(e) => &e.common,
            Self::Repay(e) => &e.common,
            Self::ReserveDataUpdated(e) => &e.common,
            Self::Supply(e) => &e.common,
            Self::Withdraw(e) => &e.common,
        }
    }
    pub fn block_number(&self) -> u64 {
        self.common().blockNumber
    }

    pub fn tx_hash(&self) -> &str {
        &self.common().txHash
    }

    pub fn id(&self) -> ObjectId {
        self.common().id
    }

    pub fn chain_id(&self) -> u64 {
        self.common().chainId
    }
    pub fn log_index(&self) -> i64 {
        self.common().logIndex
    }

    pub fn version(&self) -> i32 {
        self.common().version
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::ATokenBalanceTransfer(_) => "a-token-balance-transfer",
            Self::ATokenBurn(_) => "a-token-burn",
            Self::ATokenMint(_) => "a-token-mint",
            Self::ATokenTransfer(_) => "a-token-transfer",
            Self::Borrow(_) => "borrow",
            Self::DebtTokenBurn(_) => "debt-token-burn",
            Self::DebtTokenMint(_) => "debt-token-mint",
            Self::Repay(_) => "repay",
            Self::ReserveDataUpdated(_) => "reserve-data-updated",
            Self::Supply(_) => "supply",
            Self::Withdraw(_) => "withdraw",
        }
    }
}
