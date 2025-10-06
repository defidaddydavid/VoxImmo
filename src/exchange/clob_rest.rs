use super::types::Side;
use anyhow::Result;
use async_trait::async_trait;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use std::collections::HashMap;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct OrderRequest {
    pub token_id: String,
    pub side: Side,
    pub size: f64,
    pub price_cents: i32,
}

#[derive(Debug, Clone)]
pub struct OrderResponse {
    pub order_id: String,
}

#[async_trait]
pub trait ClobExec: Send + Sync {
    async fn place(&self, order: OrderRequest) -> Result<OrderResponse>;
    async fn cancel(&self, order_id: &str) -> Result<()>;
}

#[derive(Default)]
pub struct MockClobExec {
    orders: Mutex<HashMap<String, OrderRequest>>,
}

#[async_trait]
impl ClobExec for MockClobExec {
    async fn place(&self, order: OrderRequest) -> Result<OrderResponse> {
        let mut rng = thread_rng();
        let order_id: String = (0..8).map(|_| rng.sample(Alphanumeric) as char).collect();
        self.orders.lock().await.insert(order_id.clone(), order);
        Ok(OrderResponse { order_id })
    }

    async fn cancel(&self, order_id: &str) -> Result<()> {
        self.orders.lock().await.remove(order_id);
        Ok(())
    }
}

#[cfg(feature = "live")]
pub struct LiveClobExec;

#[cfg(feature = "live")]
#[async_trait]
impl ClobExec for LiveClobExec {
    async fn place(&self, _order: OrderRequest) -> Result<OrderResponse> {
        anyhow::bail!("Live execution not yet implemented");
    }

    async fn cancel(&self, _order_id: &str) -> Result<()> {
        anyhow::bail!("Live execution not yet implemented");
    }
}
