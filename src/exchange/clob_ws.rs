use super::types::{OrderBookSnapshot, Trade};
use crate::exchange::data_api::DataApi;
use async_stream::stream;
use std::{collections::HashMap, pin::Pin, sync::Arc, time::Duration};
use tokio::time;
use tokio_stream::Stream;

#[derive(Debug, Clone)]
pub enum MarketFeedEvent {
    OrderBook(OrderBookSnapshot),
    Trade(Trade),
}

pub type EventStream = Pin<Box<dyn Stream<Item = MarketFeedEvent> + Send>>;

pub trait MarketStream: Send + Sync {
    fn stream(&self) -> EventStream;
}

pub struct HttpPollingMarketStream<D>
where
    D: DataApi + Send + Sync + 'static,
{
    data_api: Arc<D>,
    token_ids: Vec<String>,
    interval: Duration,
}

impl<D> HttpPollingMarketStream<D>
where
    D: DataApi + Send + Sync + 'static,
{
    pub fn new(data_api: Arc<D>, token_ids: Vec<String>, interval: Duration) -> Self {
        Self {
            data_api,
            token_ids,
            interval,
        }
    }
}

impl<D> MarketStream for HttpPollingMarketStream<D>
where
    D: DataApi + Send + Sync + 'static,
{
    fn stream(&self) -> EventStream {
        let data_api = self.data_api.clone();
        let tokens = self.token_ids.clone();
        let poll_interval = self.interval;
        Box::pin(stream! {
            let mut ticker = time::interval(poll_interval);
            let mut last_trade_ts: HashMap<String, i64> = HashMap::new();
            loop {
                ticker.tick().await;
                for token in &tokens {
                    match data_api.latest_orderbook(token).await {
                        Ok(book) => yield MarketFeedEvent::OrderBook(book),
                        Err(err) => {
                            tracing::warn!(%token, error = ?err, "failed to poll orderbook");
                        }
                    }
                    match data_api.recent_trades(token).await {
                        Ok(mut trades) => {
                            trades.sort_by_key(|trade| trade.ts.timestamp_millis());
                            let watermark = last_trade_ts.get(token).copied().unwrap_or_default();
                            for trade in trades {
                                let ts_key = trade.ts.timestamp_millis();
                                if ts_key > watermark {
                                    last_trade_ts.insert(token.clone(), ts_key);
                                    yield MarketFeedEvent::Trade(trade);
                                }
                            }
                        }
                        Err(err) => {
                            tracing::warn!(%token, error = ?err, "failed to poll trades");
                        }
                    }
                }
            }
        })
    }
}
