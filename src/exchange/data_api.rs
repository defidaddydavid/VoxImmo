use super::types::{Market, MarketStatus, OrderBookSnapshot, Token, Trade};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::{header::HeaderMap, Client, Url};
use serde_json::Value;
use std::{collections::HashMap, str::FromStr, sync::Arc, time::Duration};

#[async_trait]
pub trait DataApi: Send + Sync {
    async fn list_new_markets(&self) -> Result<Vec<Market>>;
    async fn tokens_for_market(&self, market_id: &str) -> Result<Vec<Token>>;
    async fn latest_orderbook(&self, token_id: &str) -> Result<OrderBookSnapshot>;
    async fn recent_trades(&self, token_id: &str) -> Result<Vec<Trade>>;
}

#[derive(Clone)]
pub struct HttpDataApi {
    client: Client,
    base_url: Url,
    api_key: Option<String>,
    market_limit: usize,
    market_cache: tokio::sync::RwLock<HashMap<String, Value>>,
    token_to_market: tokio::sync::RwLock<HashMap<String, String>>,
}

impl HttpDataApi {
    pub fn new(
        base_url: &str,
        api_key: Option<String>,
        timeout: Duration,
        market_limit: usize,
    ) -> Result<Self> {
        let base_url = Url::from_str(base_url)
            .map_err(|err| anyhow!("invalid data api base url {base_url:?}: {err}"))?;
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .context("building http data api client")?;
        Ok(Self {
            client,
            base_url,
            api_key,
            market_limit,
            market_cache: tokio::sync::RwLock::new(HashMap::new()),
            token_to_market: tokio::sync::RwLock::new(HashMap::new()),
        })
    }

    async fn fetch_markets_raw(&self) -> Result<Vec<Value>> {
        let mut url = self
            .base_url
            .join("markets")
            .context("joining markets path")?;
        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("limit", &self.market_limit.to_string());
            qp.append_pair("archived", "false");
            qp.append_pair("order", "desc");
        }
        let response = self
            .client
            .get(url)
            .headers(self.build_headers())
            .send()
            .await
            .context("sending markets request")?
            .error_for_status()
            .context("markets request failed")?;
        let payload: Value = response.json().await.context("decoding markets json")?;
        Ok(extract_array(payload))
    }

    async fn fetch_market(&self, market_id: &str) -> Result<Value> {
        {
            let cache = self.market_cache.read().await;
            if let Some(value) = cache.get(market_id) {
                return Ok(value.clone());
            }
        }
        let url = self
            .base_url
            .join(&format!("markets/{market_id}"))
            .context("joining single market path")?;
        let response = self
            .client
            .get(url)
            .headers(self.build_headers())
            .send()
            .await
            .context("sending market detail request")?
            .error_for_status()
            .context("market detail request failed")?;
        let value: Value = response.json().await.context("decoding market json")?;
        self.market_cache
            .write()
            .await
            .insert(market_id.to_string(), value.clone());
        Ok(value)
    }

    fn build_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(key) = &self.api_key {
            if let Ok(value) = key.parse() {
                headers.insert("X-API-KEY", value);
            }
        }
        headers
    }

    async fn market_id_for_token(&self, token_id: &str) -> Result<String> {
        if let Some(existing) = self.token_to_market.read().await.get(token_id).cloned() {
            return Ok(existing);
        }

        // If not cached, attempt to find it by scanning known markets.
        let markets = self.fetch_markets_raw().await?;
        for market in markets {
            if let Some(market_id) = market.get("id").and_then(|v| v.as_str()) {
                let tokens = extract_json_string_array(market.get("clobTokenIds"));
                if tokens.iter().any(|tok| tok == token_id) {
                    self.token_to_market
                        .write()
                        .await
                        .insert(token_id.to_string(), market_id.to_string());
                    return Ok(market_id.to_string());
                }
            }
        }

        Err(anyhow!("unable to resolve market id for token {token_id}"))
    }

    fn parse_market(value: &Value) -> Option<Market> {
        let id = value.get("id")?.as_str()?.to_string();
        let question = value
            .get("question")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown question")
            .to_string();
        let category = value
            .get("category")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let created_ts = value
            .get("createdAt")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|| Utc::now());
        let resolution_ts = value
            .get("endDate")
            .or_else(|| value.get("endDateIso"))
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));
        let status = match (
            value.get("closed").and_then(|v| v.as_bool()),
            value.get("active").and_then(|v| v.as_bool()),
        ) {
            (Some(true), _) => MarketStatus::Closed,
            (Some(false), Some(true)) => MarketStatus::Open,
            _ => MarketStatus::Resolved,
        };
        Some(Market {
            id,
            question,
            category,
            created_ts,
            resolution_ts,
            status,
        })
    }

    fn parse_tokens(value: &Value) -> Vec<Token> {
        let market_id = value.get("id").and_then(|v| v.as_str()).unwrap_or_default();
        let outcomes = extract_json_string_array(value.get("outcomes"));
        let token_ids = extract_json_string_array(value.get("clobTokenIds"));
        outcomes
            .into_iter()
            .zip(token_ids.into_iter())
            .map(|(outcome, token_id)| Token {
                id: token_id,
                market_id: market_id.to_string(),
                outcome,
            })
            .collect()
    }

    fn parse_orderbook(market: &Value, token_id: &str) -> OrderBookSnapshot {
        let best_bid = market
            .get("bestBid")
            .and_then(|v| v.as_f64())
            .map(|p| (p * 100.0).round() as i32);
        let best_ask = market
            .get("bestAsk")
            .and_then(|v| v.as_f64())
            .map(|p| (p * 100.0).round() as i32);
        let liquidity = market
            .get("liquidityNum")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        OrderBookSnapshot {
            ts: Utc::now(),
            market_id: market
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            token_id: token_id.to_string(),
            best_bid_cents: best_bid,
            best_ask_cents: best_ask,
            bid_sz: liquidity / 2.0,
            ask_sz: liquidity / 2.0,
        }
    }

    fn parse_trade(market_id: &str, token_id: &str, value: &Value) -> Option<Trade> {
        let ts = value
            .get("timestamp")
            .or_else(|| value.get("createdAt"))
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))?;
        let trade_id = value
            .get("id")
            .or_else(|| value.get("transactionHash"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{}-{token_id}", ts.timestamp_millis()));
        let price = value
            .get("price")
            .and_then(|v| v.as_str().or_else(|| v.as_f64().map(|f| f.to_string())))
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let size = value
            .get("size")
            .or_else(|| value.get("amount"))
            .and_then(|v| v.as_str().or_else(|| v.as_f64().map(|f| f.to_string())))
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let is_buy = value
            .get("isBuy")
            .or_else(|| value.get("side"))
            .map(|side| match side {
                Value::Bool(flag) => *flag,
                Value::String(text) => text.eq_ignore_ascii_case("buy"),
                _ => false,
            })
            .unwrap_or(false);
        let taker = value.get("taker").and_then(|v| v.as_bool()).map(|flag| {
            if flag {
                super::types::TakerMaker::Taker
            } else {
                super::types::TakerMaker::Maker
            }
        });
        let fee_cents = value
            .get("fee")
            .and_then(|v| v.as_str().or_else(|| v.as_f64().map(|f| f.to_string())))
            .and_then(|s| s.parse::<f64>().ok())
            .map(|fee| (fee * 100.0).round() as i32)
            .unwrap_or(0);
        Some(Trade {
            ts,
            trade_id,
            market_id: market_id.to_string(),
            token_id: token_id.to_string(),
            side: if is_buy {
                super::types::Side::Buy
            } else {
                super::types::Side::Sell
            },
            size_shares: size,
            price_cents: (price * 100.0).round() as i32,
            taker_maker: taker,
            fee_cents,
            wallet: value
                .get("user")
                .or_else(|| value.get("wallet"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        })
    }
}

#[async_trait]
impl DataApi for HttpDataApi {
    async fn list_new_markets(&self) -> Result<Vec<Market>> {
        let markets_raw = self.fetch_markets_raw().await?;
        let mut markets = Vec::with_capacity(markets_raw.len());
        for value in &markets_raw {
            if let Some(market) = Self::parse_market(value) {
                markets.push(market);
            }
        }
        Ok(markets)
    }

    async fn tokens_for_market(&self, market_id: &str) -> Result<Vec<Token>> {
        let market = self.fetch_market(market_id).await?;
        let tokens = Self::parse_tokens(&market);
        {
            let mut guard = self.token_to_market.write().await;
            for token in &tokens {
                guard.insert(token.id.clone(), market_id.to_string());
            }
        }
        Ok(tokens)
    }

    async fn latest_orderbook(&self, token_id: &str) -> Result<OrderBookSnapshot> {
        let market_id = self.market_id_for_token(token_id).await?;
        let market = self.fetch_market(&market_id).await?;
        Ok(Self::parse_orderbook(&market, token_id))
    }

    async fn recent_trades(&self, token_id: &str) -> Result<Vec<Trade>> {
        let market_id = self.market_id_for_token(token_id).await?;
        let mut url = self
            .base_url
            .join("trades")
            .context("joining trades path")?;
        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("market", &market_id);
            qp.append_pair("tokenId", token_id);
            qp.append_pair("limit", "50");
        }
        let response = self
            .client
            .get(url)
            .headers(self.build_headers())
            .send()
            .await
            .context("sending trades request")?
            .error_for_status()
            .context("trades request failed")?;
        let payload: Value = response.json().await.context("decoding trades json")?;
        let mut trades = Vec::new();
        for entry in extract_array(payload) {
            if let Some(trade) = Self::parse_trade(&market_id, token_id, &entry) {
                trades.push(trade);
            }
        }
        Ok(trades)
    }
}

#[async_trait]
impl<T> DataApi for Arc<T>
where
    T: DataApi + ?Sized,
{
    async fn list_new_markets(&self) -> Result<Vec<Market>> {
        (**self).list_new_markets().await
    }

    async fn tokens_for_market(&self, market_id: &str) -> Result<Vec<Token>> {
        (**self).tokens_for_market(market_id).await
    }

    async fn latest_orderbook(&self, token_id: &str) -> Result<OrderBookSnapshot> {
        (**self).latest_orderbook(token_id).await
    }

    async fn recent_trades(&self, token_id: &str) -> Result<Vec<Trade>> {
        (**self).recent_trades(token_id).await
    }
}

fn extract_array(value: Value) -> Vec<Value> {
    match value {
        Value::Array(list) => list,
        Value::Object(map) => map
            .get("data")
            .and_then(|inner| inner.as_array().cloned())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn extract_json_string_array(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(raw)) => serde_json::from_str::<Vec<String>>(raw).unwrap_or_default(),
        Some(Value::Array(list)) => list
            .iter()
            .filter_map(|item| item.as_str().map(|s| s.to_string()))
            .collect(),
        _ => Vec::new(),
    }
}
