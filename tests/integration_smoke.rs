use polymarket_sweeper::{
    config::AppConfig,
    sim::runner::{run_simulation, SimulationRequest},
};

#[tokio::test]
async fn simulation_runs() {
    let config = AppConfig::default();
    let result = run_simulation(SimulationRequest {
        config,
        market_count: 2,
    })
    .await
    .expect("simulation");
    assert!(result.metrics_summary["markets"].as_u64().unwrap() >= 1);
}
