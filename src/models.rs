use mongodb::bson::{oid::ObjectId, DateTime, Decimal128};
use serde::{Deserialize, Serialize};

pub enum CollectionTypes {
    OrderbookDocument,
}

#[derive(Debug, Deserialize, Serialize)]
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
pub struct IntentState {
    pub exists: bool,
    pub remainingInput: Decimal128,
    pub receivedOutput: Decimal128,
    pub pendingPayment: bool,
}

#[derive(Debug, Deserialize, Serialize)]
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
    pub blockNumber: Decimal128,
}
