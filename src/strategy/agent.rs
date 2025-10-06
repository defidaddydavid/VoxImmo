use super::{
    fair_value::estimate_threshold_market,
    mispricing::{detect, MispricingAction},
    position::Position,
    quoting,
    risk::RiskManager,
};
use crate::{
    config::AppConfig,
    exchange::{
        clob_ws::{MarketFeedEvent, MarketStream},
        data_api::DataApi,
        types::{Market, OrderBookSnapshot, Token, Trade},
    },
    telemetry::Metrics,
};
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use futures::StreamExt;
use std::collections::HashMap;
use tracing::info;

#[derive(Debug, Clone)]
pub enum AgentEvent {
    MarketCreated(Market),
    Book(OrderBookSnapshot),
    Trade(Trade),
    Timer,
}

#[derive(Debug, Clone)]
pub struct AgentState {
    pub market: Market,
    pub tokens: Vec<Token>,
    pub position: Position,
    pub last_mid_cents: Option<i32>,
    pub entered_at: DateTime<Utc>,
}

impl AgentState {
    fn new(market: Market, tokens: Vec<Token>) -> Self {
        Self {
            market,
            tokens,
            position: Position::default(),
            last_mid_cents: None,
            entered_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IngestStats {
    pub rows: usize,
}

pub async fn run_ingest(
    _config: &AppConfig,
    api: &impl DataApi,
    _days: u32,
) -> Result<IngestStats> {
    let markets = api.list_new_markets().await?;
    let mut rows = markets.len();
    for market in &markets {
        rows += api.tokens_for_market(&market.id).await?.len();
    }
    Ok(IngestStats { rows })
}

pub async fn run_agents<D, S>(
    config: AppConfig,
    data_api: D,
    stream_provider: S,
    metrics: Metrics,
) -> Result<()>
where
    D: DataApi + 'static,
    S: MarketStream + 'static,
{
    let markets = data_api.list_new_markets().await?;
    let mut agents: HashMap<String, AgentState> = HashMap::new();
    for market in markets {
        if within_window(&market, config.new_market.window_minutes) {
            let tokens = data_api.tokens_for_market(&market.id).await?;
            agents.insert(market.id.clone(), AgentState::new(market, tokens));
        }
    }

    let mut stream = stream_provider.stream();
    let mut risk = RiskManager::new(config.risk.clone(), 10_000.0);
    let mut processed = 0u32;
    while let Some(event) = stream.next().await {
        match event {
            MarketFeedEvent::OrderBook(snapshot) => {
                if let Some(state) = agents.get_mut(&snapshot.market_id) {
                    handle_orderbook(state, &config, snapshot, &mut risk, &metrics);
                }
            }
            MarketFeedEvent::Trade(trade) => {
                if let Some(state) = agents.get_mut(&trade.market_id) {
                    handle_trade(state, trade, &metrics);
                }
            }
        }
        processed += 1;
        if processed > 200 {
            break;
        }
    }

    metrics
        .drawdown_pct
        .set((risk.max_drawdown_observed * 100.0) as i64);
    Ok(())
}

fn within_window(market: &Market, window_minutes: u64) -> bool {
    let age = Utc::now() - market.created_ts;
    age <= Duration::minutes(window_minutes as i64)
}

fn handle_orderbook(
    state: &mut AgentState,
    config: &AppConfig,
    snapshot: OrderBookSnapshot,
    risk: &mut RiskManager,
    metrics: &Metrics,
) {
    let mid = match (snapshot.best_bid_cents, snapshot.best_ask_cents) {
        (Some(bid), Some(ask)) => ((bid + ask) / 2) as i32,
        (Some(bid), None) => bid,
        (None, Some(ask)) => ask,
        _ => return,
    };
    state.last_mid_cents = Some(mid);
    let probability = (mid as f64 / 100.0).clamp(0.01, 0.99);
    let days_to_resolution = state
        .market
        .resolution_ts
        .map(|ts| (ts - Utc::now()).num_seconds().max(0) as f64 / 86_400.0)
        .unwrap_or(30.0);
    let fair = estimate_threshold_market(
        45_000.0,
        40_000.0,
        (days_to_resolution / 365.0).max(1.0 / 365.0),
        Some(0.6),
        5.0,
    );
    let signal = detect(probability, fair, config.mispricing.deviation_threshold);
    if signal.action != MispricingAction::Flat {
        metrics.orders_sent.inc();
        info!(market = %state.market.id, ?signal, "mispricing detected");
    }

    let position_notional = state.position.net_qty() * (mid as f64 / 100.0);
    let capacity = (config.risk.max_position_usd_per_market - position_notional.abs()).max(0.0);
    if let Some(plan) = quoting::plan_quotes(mid, 2.0, &config.quoting, position_notional, capacity)
    {
        info!(market = %state.market.id, ?plan, "updating quotes");
    }

    risk.record_equity(10_000.0 - position_notional.abs() * 0.1);
}

fn handle_trade(state: &mut AgentState, trade: Trade, metrics: &Metrics) {
    state
        .position
        .apply_fill(trade.side, trade.size_shares, trade.price_cents);
    metrics.fills.inc();
    metrics.pnl_cents.set(state.position.realized_pnl_cents);
}
