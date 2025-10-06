use super::types::{Market, MarketStatus, OrderBookSnapshot, Token, Trade};
use anyhow::Result;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use rand::{distributions::Alphanumeric, rngs::StdRng, Rng, SeedableRng};
use std::collections::HashMap;

#[async_trait]
pub trait DataApi: Send + Sync {
    async fn list_new_markets(&self) -> Result<Vec<Market>>;
    async fn tokens_for_market(&self, market_id: &str) -> Result<Vec<Token>>;
    async fn latest_orderbook(&self, token_id: &str) -> Result<OrderBookSnapshot>;
    async fn recent_trades(&self, token_id: &str) -> Result<Vec<Trade>>;
}

#[derive(Default, Clone)]
pub struct MockDataApi {
    rng: StdRng,
    markets: Vec<Market>,
    tokens: HashMap<String, Vec<Token>>,
}

impl MockDataApi {
    pub fn new_with_seed(seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let markets = (0..5)
            .map(|idx| Market {
                id: format!("m{}", idx),
                question: format!(
                    "Will BTC close above {} by end of week?",
                    40_000 + idx * 500
                ),
                category: Some("Crypto".to_string()),
                created_ts: Utc::now() - Duration::minutes(10 + idx as i64),
                resolution_ts: Some(Utc::now() + Duration::days(7)),
                status: MarketStatus::Open,
            })
            .collect::<Vec<_>>();
        let tokens = markets
            .iter()
            .map(|m| {
                let yes = Token {
                    id: format!("{}-yes", m.id),
                    market_id: m.id.clone(),
                    outcome: "Yes".into(),
                };
                let no = Token {
                    id: format!("{}-no", m.id),
                    market_id: m.id.clone(),
                    outcome: "No".into(),
                };
                (m.id.clone(), vec![yes, no])
            })
            .collect();
        Self {
            rng,
            markets,
            tokens,
        }
    }
}

#[async_trait]
impl DataApi for MockDataApi {
    async fn list_new_markets(&self) -> Result<Vec<Market>> {
        Ok(self.markets.clone())
    }

    async fn tokens_for_market(&self, market_id: &str) -> Result<Vec<Token>> {
        Ok(self.tokens.get(market_id).cloned().unwrap_or_default())
    }

    async fn latest_orderbook(&self, token_id: &str) -> Result<OrderBookSnapshot> {
        let mut rng = self.rng.clone();
        let mid = rng.gen_range(20..80) as i32;
        Ok(OrderBookSnapshot {
            ts: Utc::now(),
            market_id: token_id.split('-').next().unwrap_or("mock").to_string(),
            token_id: token_id.to_string(),
            best_bid_cents: Some(mid - 1),
            best_ask_cents: Some(mid + 1),
            bid_sz: rng.gen_range(50.0..100.0),
            ask_sz: rng.gen_range(50.0..100.0),
        })
    }

    async fn recent_trades(&self, token_id: &str) -> Result<Vec<Trade>> {
        let mut rng = self.rng.clone();
        let trades = (0..3)
            .map(|idx| Trade {
                ts: Utc::now() - Duration::seconds(idx * 10),
                trade_id: format!("{}-{}", token_id, rng.sample(Alphanumeric) as char),
                market_id: token_id.split('-').next().unwrap_or("mock").to_string(),
                token_id: token_id.to_string(),
                side: if idx % 2 == 0 {
                    super::types::Side::Buy
                } else {
                    super::types::Side::Sell
                },
                size_shares: rng.gen_range(10.0..50.0),
                price_cents: rng.gen_range(10..90),
                taker_maker: None,
                fee_cents: 2,
                wallet: None,
            })
            .collect();
        Ok(trades)
    }
}

impl Default for MockDataApi {
    fn default() -> Self {
        Self::new_with_seed(42)
    }
}
