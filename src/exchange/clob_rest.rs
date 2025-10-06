use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::{Client, Url};
use serde_json::Value;
use std::{str::FromStr, time::Duration};

#[derive(Debug, Clone)]
pub struct OrderRequest {
    pub payload: Value,
}

impl OrderRequest {
    pub fn new(payload: Value) -> Self {
        Self { payload }
    }
}

#[derive(Debug, Clone)]
pub struct OrderResponse {
    pub order_id: String,
    pub raw: Value,
}

#[async_trait]
pub trait ClobExec: Send + Sync {
    async fn place(&self, order: OrderRequest) -> Result<OrderResponse>;
    async fn cancel(&self, order_id: &str) -> Result<()>;
}

#[derive(Clone)]
pub struct HttpClobExec {
    client: Client,
    base_url: Url,
    api_key: Option<String>,
}

impl HttpClobExec {
    pub fn new(base_url: &str, api_key: Option<String>, timeout: Duration) -> Result<Self> {
        let base_url = Url::from_str(base_url)
            .map_err(|err| anyhow!("invalid clob url {base_url:?}: {err}"))?;
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .context("building clob http client")?;
        Ok(Self {
            client,
            base_url,
            api_key,
        })
    }

    fn apply_headers(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(key) = &self.api_key {
            request.header("X-API-KEY", key)
        } else {
            request
        }
    }
}

#[async_trait]
impl ClobExec for HttpClobExec {
    async fn place(&self, order: OrderRequest) -> Result<OrderResponse> {
        let url = self
            .base_url
            .join("orders")
            .context("joining orders path")?;
        let response = self
            .apply_headers(self.client.post(url))
            .json(&order.payload)
            .send()
            .await
            .context("sending place order request")?
            .error_for_status()
            .context("order placement failed")?;
        let body: Value = response.json().await.context("decoding order response")?;
        let order_id = body
            .get("orderId")
            .or_else(|| body.get("order_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("missing order id in response"))?;
        Ok(OrderResponse {
            order_id: order_id.to_string(),
            raw: body,
        })
    }

    async fn cancel(&self, order_id: &str) -> Result<()> {
        let url = self
            .base_url
            .join(&format!("orders/{order_id}"))
            .context("joining cancel path")?;
        self.apply_headers(self.client.delete(url))
            .send()
            .await
            .context("sending cancel request")?
            .error_for_status()
            .context("cancel request failed")?;
        Ok(())
    }
}

#[cfg(feature = "live")]
pub type LiveClobExec = HttpClobExec;
