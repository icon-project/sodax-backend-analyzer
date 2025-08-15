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

#[derive(Debug, Deserialize, Serialize)]
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

#[derive(Debug, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct IntentState {
    pub exists: bool,
    pub remainingInput: Decimal128,
    pub receivedOutput: Decimal128,
    pub pendingPayment: bool,
}

#[derive(Debug, Deserialize, Serialize)]
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
