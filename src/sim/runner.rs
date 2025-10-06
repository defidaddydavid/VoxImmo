use super::{feed::HistoricalFeed, fill_engine::FillEngine};
use crate::{
    config::AppConfig,
    exchange::types::Side,
    sim::feed::SimEvent,
    strategy::{
        fair_value::estimate_threshold_market,
        mispricing::{detect, MispricingAction},
        position::Position,
    },
};
use anyhow::Result;
use polars::prelude::*;
use serde::Serialize;
use serde_json::json;
use std::{fs::create_dir_all, path::Path};

#[derive(Debug, Clone, Serialize)]
pub struct SimulationSummary {
    pub market_id: String,
    pub realized_pnl_cents: i64,
    pub fills: usize,
    pub max_position: f64,
}

#[derive(Debug, Clone)]
pub struct SimulationRequest {
    pub config: AppConfig,
    pub market_count: usize,
}

#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub metrics_summary: serde_json::Value,
}

pub async fn run_simulation(request: SimulationRequest) -> Result<SimulationResult> {
    let feed = HistoricalFeed::new(1234);
    let mut rows: Vec<SimulationSummary> = Vec::new();
    let engine = FillEngine::new(request.config.sim.clone());

    for idx in 0..request.market_count.max(1) {
        let market_id = format!("sim-market-{}", idx);
        let token_id = format!("{}-yes", market_id);
        let events = feed.mock_events(&market_id, &token_id, 60);
        let mut position = Position::default();
        let mut fills = 0usize;
        let mut max_position = 0.0;
        process_events(
            &events,
            &engine,
            &request.config,
            &mut position,
            &mut fills,
            &mut max_position,
        );
        rows.push(SimulationSummary {
            market_id,
            realized_pnl_cents: position.realized_pnl_cents,
            fills,
            max_position,
        });
    }

    write_csv(&rows)?;
    let avg_pnl: f64 = rows
        .iter()
        .map(|r| r.realized_pnl_cents as f64)
        .sum::<f64>()
        / rows.len() as f64;
    let metrics_summary = json!({
        "markets": rows.len(),
        "avg_realized_pnl_cents": avg_pnl,
        "avg_fills": rows.iter().map(|r| r.fills as f64).sum::<f64>() / rows.len() as f64,
    });

    Ok(SimulationResult { metrics_summary })
}

fn process_events(
    events: &[SimEvent],
    engine: &FillEngine,
    config: &AppConfig,
    position: &mut Position,
    fills: &mut usize,
    max_position: &mut f64,
) {
    for event in events {
        if let Some(book) = &event.book {
            let mid = match (book.best_bid_cents, book.best_ask_cents) {
                (Some(bid), Some(ask)) => (bid + ask) / 2,
                (Some(bid), None) => bid,
                (None, Some(ask)) => ask,
                _ => continue,
            };
            let probability = mid as f64 / 100.0;
            let fair = estimate_threshold_market(45_000.0, 40_000.0, 0.1, Some(0.6), 5.0);
            let signal = detect(probability, fair, config.mispricing.deviation_threshold);
            if signal.action == MispricingAction::BuyYes {
                let outcome = engine.sweep(
                    Side::Buy,
                    config.quoting.clip_usd / (mid as f64 / 100.0),
                    mid,
                );
                position.apply_fill(
                    Side::Buy,
                    outcome.filled,
                    outcome.avg_price_cents.round() as i32,
                );
                *fills += 1;
            } else if signal.action == MispricingAction::SellYes {
                let outcome = engine.sweep(
                    Side::Sell,
                    config.quoting.clip_usd / (mid as f64 / 100.0),
                    mid,
                );
                position.apply_fill(
                    Side::Sell,
                    outcome.filled,
                    outcome.avg_price_cents.round() as i32,
                );
                *fills += 1;
            }
        }
        if let Some(trade) = &event.trade {
            position.apply_fill(trade.side, trade.size_shares, trade.price_cents);
            *fills += 1;
        }
        *max_position = max_position.max(position.net_qty().abs());
    }
}

fn write_csv(rows: &[SimulationSummary]) -> Result<()> {
    create_dir_all(Path::new("sim_output"))?;
    let market_ids: Vec<&str> = rows.iter().map(|r| r.market_id.as_str()).collect();
    let pnl: Vec<i64> = rows.iter().map(|r| r.realized_pnl_cents).collect();
    let fills: Vec<usize> = rows.iter().map(|r| r.fills).collect();
    let max_pos: Vec<f64> = rows.iter().map(|r| r.max_position).collect();

    let df = DataFrame::new(vec![
        Series::new("market_id", market_ids),
        Series::new("realized_pnl_cents", pnl),
        Series::new("fills", fills),
        Series::new("max_position", max_pos),
    ])?;
    let path = Path::new("sim_output/summary.csv");
    let mut file = std::fs::File::create(path)?;
    CsvWriter::new(&mut file).finish(&df)?;
    Ok(())
}
