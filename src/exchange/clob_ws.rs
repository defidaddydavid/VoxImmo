use super::types::{OrderBookSnapshot, Trade};
use chrono::Utc;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::{pin::Pin, time::Duration};
use tokio::time::interval;
use tokio_stream::{wrappers::IntervalStream, Stream, StreamExt};

#[derive(Debug, Clone)]
pub enum MarketFeedEvent {
    OrderBook(OrderBookSnapshot),
    Trade(Trade),
}

pub type EventStream = Pin<Box<dyn Stream<Item = MarketFeedEvent> + Send>>;

pub trait MarketStream: Send + Sync {
    fn stream(&self) -> EventStream;
}

#[derive(Clone)]
pub struct MockMarketStream {
    seed: u64,
}

impl Default for MockMarketStream {
    fn default() -> Self {
        Self { seed: 7 }
    }
}

impl MarketStream for MockMarketStream {
    fn stream(&self) -> EventStream {
        let mut rng = StdRng::seed_from_u64(self.seed);
        let interval = IntervalStream::new(interval(Duration::from_millis(200)));
        Box::pin(interval.map(move |_| {
            if rng.gen_bool(0.6) {
                MarketFeedEvent::OrderBook(OrderBookSnapshot {
                    ts: Utc::now(),
                    market_id: "m0".to_string(),
                    token_id: "m0-yes".to_string(),
                    best_bid_cents: Some(rng.gen_range(40..60)),
                    best_ask_cents: Some(rng.gen_range(60..80)),
                    bid_sz: rng.gen_range(20.0..80.0),
                    ask_sz: rng.gen_range(20.0..80.0),
                })
            } else {
                MarketFeedEvent::Trade(Trade {
                    ts: Utc::now(),
                    trade_id: format!("t-{}", rng.gen::<u32>()),
                    market_id: "m0".to_string(),
                    token_id: "m0-yes".to_string(),
                    side: if rng.gen_bool(0.5) {
                        super::types::Side::Buy
                    } else {
                        super::types::Side::Sell
                    },
                    size_shares: rng.gen_range(5.0..25.0),
                    price_cents: rng.gen_range(30..90),
                    taker_maker: None,
                    fee_cents: 1,
                    wallet: None,
                })
            }
        }))
    }
}
