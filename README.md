# Polymarket Sweeper

This repository implements a fully mocked trading system for Polymarket markets. It provides the scaffolding for discovering new markets, estimating fair value, detecting mispricing, simulating executions, and running dry-run agents. The code is written in Rust 2021 and is structured so that real endpoints can be added later with minimal changes.

## Quickstart

1. Install Rust (1.74+) and PostgreSQL.
2. Copy `.env.example` to `.env` and edit as needed.
3. Run migrations:
   ```bash
   cargo sqlx migrate run
   ```
4. Run the simulator with default config:
   ```bash
   cargo run -- sim --config configs/default.toml --markets 3
   ```
5. Run the dry-run agent loop (mocked websocket + data api):
   ```bash
   cargo run -- run --config configs/dev.toml
   ```

## Project Structure

The project is split into modules that mirror the production system:

- `config`: configuration loading and validation.
- `telemetry`: tracing and Prometheus metrics exporter using Axum.
- `db`: sqlx connection management, models, and query helpers.
- `exchange`: data API and CLOB execution traits with mock implementations.
- `strategy`: fair value estimator, mispricing detector, risk manager, quoting logic, and asynchronous market agent.
- `sim`: mocked feed, fill engine, and simulation runner producing CSV KPI reports using Polars.
- `backtest`: pre-defined experiment grids for the simulator.
- `util`: time and math helpers shared across modules.
- `tests`: unit and property tests exercising strategy invariants and accounting logic.

## Running Tests and Lints

```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test
```

## Live Trading

Live order placement is gated behind the `live` feature flag. The provided code exposes trait boundaries (`DataApi`, `ClobExec`) that must be implemented with real Polymarket endpoints before enabling `--live`. Environment variables are only parsed in live mode to avoid accidental key leakage.

## Simulator Output

The simulator runs a configurable grid search and writes KPI CSV files to `./sim_output/`. Each run logs realized spread, PnL, max drawdown, and latency metrics. The output can be inspected with standard tools or loaded back into Polars for further analysis.

## Safety

- No private keys are included.
- Graceful shutdown cancels open quotes, flushes logs, and stops the metrics server.
- Risk management ensures per-market caps, portfolio exposure limits, and drawdown kill-switches are respected even in the simulator.
