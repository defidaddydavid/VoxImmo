use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MarketRow {
    pub id: String,
    pub question: String,
    pub category: Option<String>,
    pub created_ts: DateTime<Utc>,
    pub resolution_ts: Option<DateTime<Utc>>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TokenRow {
    pub id: String,
    pub market_id: String,
    pub outcome: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TradeRow {
    pub ts: DateTime<Utc>,
    pub trade_id: String,
    pub market_id: String,
    pub token_id: String,
    pub side: String,
    pub size_shares: f64,
    pub price_cents: i32,
    pub taker_maker: Option<String>,
    pub fee_cents: Option<i32>,
    pub wallet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OrderBookSnapshotRow {
    pub ts: DateTime<Utc>,
    pub market_id: String,
    pub token_id: String,
    pub best_bid_cents: Option<i32>,
    pub best_ask_cents: Option<i32>,
    pub bid_sz: Option<f64>,
    pub ask_sz: Option<f64>,
}
