use crate::exchange::types::{OrderBookSnapshot, Side, Trade};
use chrono::{Duration, Utc};
use rand::{rngs::StdRng, Rng, SeedableRng};

#[derive(Debug, Clone)]
pub struct SimEvent {
    pub book: Option<OrderBookSnapshot>,
    pub trade: Option<Trade>,
}

#[derive(Debug, Clone)]
pub struct HistoricalFeed {
    seed: u64,
}

impl HistoricalFeed {
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }

    pub fn mock_events(&self, market_id: &str, token_id: &str, len: usize) -> Vec<SimEvent> {
        let mut rng = StdRng::seed_from_u64(self.seed);
        (0..len)
            .map(|idx| {
                let mid = 50 + ((idx as i32 * 2) % 10) + rng.gen_range(-2..3);
                let book = OrderBookSnapshot {
                    ts: Utc::now() + Duration::seconds(idx as i64),
                    market_id: market_id.to_string(),
                    token_id: token_id.to_string(),
                    best_bid_cents: Some(mid - 2),
                    best_ask_cents: Some(mid + 2),
                    bid_sz: rng.gen_range(20.0..80.0),
                    ask_sz: rng.gen_range(20.0..80.0),
                };
                let trade = if rng.gen_bool(0.4) {
                    Some(Trade {
                        ts: book.ts,
                        trade_id: format!("{}-{}", market_id, idx),
                        market_id: market_id.to_string(),
                        token_id: token_id.to_string(),
                        side: if rng.gen_bool(0.5) {
                            Side::Buy
                        } else {
                            Side::Sell
                        },
                        size_shares: rng.gen_range(5.0..20.0),
                        price_cents: mid,
                        taker_maker: None,
                        fee_cents: 1,
                        wallet: None,
                    })
                } else {
                    None
                };
                SimEvent {
                    book: Some(book),
                    trade,
                }
            })
            .collect()
    }
}
