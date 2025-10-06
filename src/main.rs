use clap::{Parser, Subcommand};
use polymarket_sweeper::{
    config::AppConfig,
    exchange::{clob_ws::MockMarketStream, data_api::MockDataApi},
    sim::runner::{run_simulation, SimulationRequest},
    strategy::agent,
    telemetry,
};
use serde_json::json;
use std::path::PathBuf;
use tracing::info;

#[derive(Parser, Debug)]
#[command(author, version, about = "Polymarket mispricing sweeper", long_about = None)]
struct Cli {
    /// Path to configuration file
    #[arg(long)]
    config: Option<PathBuf>,

    /// Enable live order placement (requires `--features live`)
    #[arg(long)]
    live: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Discover,
    Ingest { days: u32 },
    Sim { markets: usize },
    Run,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    telemetry::init_tracing();

    let cli = Cli::parse();
    let config_path = cli.config.as_ref().map(|p| p.as_path());
    let app_config = AppConfig::load(config_path)?;

    if cli.live && !cfg!(feature = "live") {
        anyhow::bail!("live mode requested but binary built without `live` feature");
    }

    match cli.command {
        Commands::Discover => {
            info!("command" = "discover", "starting new market discovery");
            let api = MockDataApi::default();
            let markets = api.list_new_markets().await?;
            println!("{}", serde_json::to_string_pretty(&markets)?);
        }
        Commands::Ingest { days } => {
            info!("command" = "ingest", %days, "starting ingest run");
            let api = MockDataApi::default();
            let stats = agent::run_ingest(&app_config, &api, days).await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({"ingested_days": days, "rows": stats.rows}))?
            );
        }
        Commands::Sim { markets } => {
            info!("command" = "sim", markets, "running simulator");
            let request = SimulationRequest {
                config: app_config.clone(),
                market_count: markets,
            };
            let result = run_simulation(request).await?;
            println!("{}", serde_json::to_string_pretty(&result.metrics_summary)?);
        }
        Commands::Run => {
            info!("command" = "run", "starting agent loop");
            let metrics = telemetry::Metrics::default();
            let _server = telemetry::spawn_metrics_server(
                metrics.registry().clone(),
                app_config.telemetry.prom_port,
            )
            .await?;
            let api = MockDataApi::default();
            let stream = MockMarketStream::default();
            agent::run_agents(app_config, api, stream, metrics).await?;
        }
    }

    Ok(())
}
