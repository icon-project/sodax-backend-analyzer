#[derive(Debug, Clone)]
pub enum Flag {
  Help,
  ReserveToken(String),
  AToken(String),
  DebtToken(String),
  UserPosition(String),
  AllTokens,
  BalanceOf(String),
  LastBlock,
  Orderbook,
  Scaled,
  ValidateUserSupply(String),
  ValidateUserBorrow(String),
  ValidateTokenSupply,
  ValidateTokenBorrow,
  ValidateUserAll(String),
  ValidateUsersAll,
  ValidateTokenAll,
  ValidateAll,
  TimestampCoverage,
  ValidateTimestamps(Option<String>),
  GetAllUsers,
  GetAllReserves,
  GetAllATokens,
  GetAllDebtTokens,
  GetTokenEvents(String),
  GetUserEvents(String),
  ValidateReserveIndexes(String),
  ValidateAllReserveIndexes,
  CalculateFromEvents(String),
  Block(u64),
  InspectUserPosition(String),
  ValidateFromEvents(String),
  ValidateFromEventsAll,
  ValidatePartnerAsset,
  Partner(String),
  Json,
  Threshold(f64),
}
#[derive(Debug, Clone)]
pub struct EntryState {
  pub database_amount: u128,
  pub on_chain_amount: u128,
  pub difference: u128,
  pub percentage: f64,
}
impl EntryState {
  pub fn new(database_amount: u128, on_chain_amount: u128) -> Self {
    let difference = database_amount.abs_diff(on_chain_amount);

    // Handle division by zero and edge cases
    let percentage = if on_chain_amount == 0 {
      if database_amount == 0 {
        0.0 // Both are 0, so 0% difference
      } else {
        100.0 // Database has amount but on-chain is 0, so 100% difference
      }
    } else if database_amount == 0 {
      100.0 // Database is 0 but on-chain has amount, so 100% difference
    } else {
      (difference as f64 / on_chain_amount as f64) * 100.0
    };

    EntryState {
      database_amount,
      on_chain_amount,
      difference,
      percentage,
    }
  }
}

#[derive(Debug, Clone)]
pub struct UserPositionValidation {
  pub reserve_address: String,
  pub supply: EntryState,
  pub borrow: EntryState,
  pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UserEntryState {
  pub user_address: String,
  pub positions: Vec<UserPositionValidation>,
}

impl UserEntryState {
  pub fn new(user_address: String) -> Self {
    UserEntryState {
      user_address,
      positions: Vec::new(),
    }
  }
}
#[derive(Debug, Clone)]
pub struct ReserveEntryState {
  pub reserve_address: String,
  pub supply: EntryState,
  pub borrow: EntryState,
  pub error: Option<String>,
}

impl ReserveEntryState {
  pub fn new(reserve_address: String) -> Self {
    ReserveEntryState {
      reserve_address,
      supply: EntryState::new(0, 0),
      borrow: EntryState::new(0, 0),
      error: None,
    }
  }

  pub fn with_error(reserve_address: String, error: String) -> Self {
    ReserveEntryState {
      reserve_address,
      supply: EntryState::new(0, 0),
      borrow: EntryState::new(0, 0),
      error: Some(error),
    }
  }
}

#[derive(Debug, Clone)]
pub struct Collections {
  pub orderbook: &'static str,
  pub money_market_events: &'static str,
  pub money_market_metadata: &'static str,
  pub user_positions: &'static str,
  pub reserve_tokens: &'static str,
  pub orderbook_metadata: &'static str,
  pub wallet_factory_events: &'static str,
  pub intent_events: &'static str,
  pub eventlog_progress_metadata: &'static str,
  pub solver_volume: &'static str,
  pub user_balance_events: &'static str,
  pub partner_asset: &'static str,
}

impl Default for Collections {
  fn default() -> Self {
    Collections::new()
  }
}

impl Collections {
  pub fn new() -> Self {
    Collections {
      orderbook: "orderbook",
      money_market_events: "money_market_events",
      money_market_metadata: "money_market_metadata",
      user_positions: "user_positions",
      reserve_tokens: "reserve_tokens",
      orderbook_metadata: "orderbook_metadata",
      wallet_factory_events: "wallet_factory_events",
      intent_events: "intentEvents",
      eventlog_progress_metadata: "event_log_progress_metadata",
      solver_volume: "solver_volume",
      user_balance_events: "user_balance_events",
      partner_asset: "partner_asset",
    }
  }
}

#[derive(Debug)]
pub enum ReserveTokenField {
  Reserve,
  AToken,
  VariableDebtToken,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FlagType {
  ReserveToken,
  AToken,
  DebtToken,
  UserPosition,
  BalanceOf,
  ValidateUserSupply,
  ValidateUserBorrow,
  ValidateUserAll,
  ValidateTimestamps,
  ValidateReserveIndexes,
  GetTokenEvents,
  GetUserEvents,
  CalculateFromEvents,
  Block,
  InspectUserPosition,
  ValidateFromEvents,
  Partner,
}

#[derive(Debug, Clone)]
pub struct ThreeWayComparison {
  pub from_events: u128,
  pub from_db: u128,
  pub on_chain: u128,
  pub events_vs_chain_diff: u128,
  pub events_vs_chain_pct: f64,
  pub db_vs_chain_diff: u128,
  pub db_vs_chain_pct: f64,
  pub events_vs_db_diff: u128,
  pub events_vs_db_pct: f64,
}

impl ThreeWayComparison {
  pub fn new(from_events: u128, from_db: u128, on_chain: u128) -> Self {
    let events_vs_chain_diff = from_events.abs_diff(on_chain);
    let db_vs_chain_diff = from_db.abs_diff(on_chain);
    let events_vs_db_diff = from_events.abs_diff(from_db);

    let pct = |diff: u128, base: u128| -> f64 {
      if base == 0 {
        if diff == 0 { 0.0 } else { 100.0 }
      } else {
        (diff as f64 / base as f64) * 100.0
      }
    };

    ThreeWayComparison {
      from_events,
      from_db,
      on_chain,
      events_vs_chain_diff,
      events_vs_chain_pct: pct(events_vs_chain_diff, on_chain),
      db_vs_chain_diff,
      db_vs_chain_pct: pct(db_vs_chain_diff, on_chain),
      events_vs_db_diff,
      events_vs_db_pct: pct(events_vs_db_diff, from_db),
    }
  }

  pub fn has_mismatch(&self, threshold_pct: f64) -> bool {
    self.events_vs_chain_pct > threshold_pct
      || self.db_vs_chain_pct > threshold_pct
      || self.events_vs_db_pct > threshold_pct
  }
}

#[derive(Debug, Clone)]
pub struct EventValidationResult {
  pub user_address: String,
  pub reserve_address: String,
  pub supply: Option<ThreeWayComparison>,
  pub borrow: Option<ThreeWayComparison>,
  pub error: Option<String>,
}
