use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MarketStatus {
    Open,
    Closed,
    Resolved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    pub id: String,
    pub question: String,
    pub category: Option<String>,
    pub created_ts: DateTime<Utc>,
    pub resolution_ts: Option<DateTime<Utc>>,
    pub status: MarketStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub id: String,
    pub market_id: String,
    pub outcome: String,
}

#[derive(Debug, Clone)]
pub struct OrderBookSnapshot {
    pub ts: DateTime<Utc>,
    pub market_id: String,
    pub token_id: String,
    pub best_bid_cents: Option<i32>,
    pub best_ask_cents: Option<i32>,
    pub bid_sz: f64,
    pub ask_sz: f64,
}

#[derive(Debug, Clone)]
pub struct Trade {
    pub ts: DateTime<Utc>,
    pub trade_id: String,
    pub market_id: String,
    pub token_id: String,
    pub side: Side,
    pub size_shares: f64,
    pub price_cents: i32,
    pub taker_maker: Option<TakerMaker>,
    pub fee_cents: i32,
    pub wallet: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

impl Side {
    pub fn invert(self) -> Self {
        match self {
            Side::Buy => Side::Sell,
            Side::Sell => Side::Buy,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TakerMaker {
    Taker,
    Maker,
}
