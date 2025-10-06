# Polymarket Sweeper

This repository implements a production-ready Polymarket trading system. It discovers newly listed markets via the public Gamma Data API, computes advanced fair-value estimates, detects mispricing, and can operate agents that poll live order book and trade data. Real CLOB REST endpoints are wired via authenticated HTTP clients so that once API credentials and signing infrastructure are supplied, orders can be placed without further code changes.

## Quickstart

1. Install Rust (1.74+) and PostgreSQL.
2. Copy `.env.example` to `.env` and set:
   - `POLYMARKET__EXCHANGE__DATA_API_BASE` (defaults to `https://gamma-api.polymarket.com/`).
   - `POLYMARKET__EXCHANGE__API_KEY` with your Polymarket CLOB API key.
   - Optional polling interval overrides.
3. Run migrations:
   ```bash
   cargo sqlx migrate run
   ```
4. Run the simulator with default config:
   ```bash
   cargo run -- sim --config configs/default.toml --markets 3
   ```
5. Run the dry-run agent loop (real HTTP polling of live markets):
   ```bash
   cargo run -- run --config configs/dev.toml
   ```

## Project Structure

The project is split into modules that mirror the production system:

- `config`: configuration loading and validation.
- `telemetry`: tracing and Prometheus metrics exporter using Axum.
- `db`: sqlx connection management, models, and query helpers.
- `exchange`: data API HTTP client, polling market stream, and authenticated CLOB execution HTTP client.
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

`ClobExec` now issues authenticated REST requests directly against the Polymarket CLOB. Provide a signed order payload via `OrderRequest::new` and compile with the `live` feature to route real orders. When running live you must also inject private-key based signatures (see Polymarket documentation) and supply your API key via configuration or environment variables.

## Simulator Output

The simulator runs a configurable grid search and writes KPI CSV files to `./sim_output/`. Each run logs realized spread, PnL, max drawdown, and latency metrics. The output can be inspected with standard tools or loaded back into Polars for further analysis.

## Safety

- No private keys are included.
- Graceful shutdown cancels open quotes, flushes logs, and stops the metrics server.
- Risk management ensures per-market caps, portfolio exposure limits, and drawdown kill-switches are respected even in the simulator.
