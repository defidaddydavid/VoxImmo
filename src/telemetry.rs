use axum::{http::StatusCode, routing::get, Router};
use prometheus::{Encoder, IntCounter, IntGauge, Registry, TextEncoder};
use std::{net::SocketAddr, sync::Arc};
use tokio::{sync::oneshot, task::JoinHandle};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Clone)]
pub struct Metrics {
    registry: Arc<Registry>,
    pub orders_sent: IntCounter,
    pub cancels: IntCounter,
    pub fills: IntCounter,
    pub pnl_cents: IntGauge,
    pub realized_spread_bps: IntGauge,
    pub drawdown_pct: IntGauge,
}

impl Default for Metrics {
    fn default() -> Self {
        let registry = Registry::new();
        let orders_sent =
            IntCounter::new("orders_sent", "Total orders submitted").expect("counter");
        let cancels = IntCounter::new("cancels", "Total order cancels").expect("counter");
        let fills = IntCounter::new("fills", "Total fills observed").expect("counter");
        let pnl_cents = IntGauge::new("pnl_cents", "Current PnL in cents").expect("gauge");
        let realized_spread_bps =
            IntGauge::new("realized_spread_bps", "Realized spread in basis points").expect("gauge");
        let drawdown_pct =
            IntGauge::new("drawdown_pct", "Drawdown percentage scaled by 100").expect("gauge");

        registry
            .register(Box::new(orders_sent.clone()))
            .expect("register");
        registry
            .register(Box::new(cancels.clone()))
            .expect("register");
        registry
            .register(Box::new(fills.clone()))
            .expect("register");
        registry
            .register(Box::new(pnl_cents.clone()))
            .expect("register");
        registry
            .register(Box::new(realized_spread_bps.clone()))
            .expect("register");
        registry
            .register(Box::new(drawdown_pct.clone()))
            .expect("register");

        Self {
            registry: Arc::new(registry),
            orders_sent,
            cancels,
            fills,
            pnl_cents,
            realized_spread_bps,
            drawdown_pct,
        }
    }
}

impl Metrics {
    pub fn registry(&self) -> &Arc<Registry> {
        &self.registry
    }
}

pub struct MetricsServerHandle {
    shutdown: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

impl MetricsServerHandle {
    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        let _ = self.task.await;
    }
}

pub async fn spawn_metrics_server(
    registry: Arc<Registry>,
    port: u16,
) -> anyhow::Result<MetricsServerHandle> {
    let (tx, rx) = oneshot::channel();
    let app = Router::new().route(
        "/metrics",
        get(move || {
            let registry = registry.clone();
            async move {
                let metric_families = registry.gather();
                let mut buffer = Vec::new();
                TextEncoder::new()
                    .encode(&metric_families, &mut buffer)
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                String::from_utf8(buffer).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
            }
        }),
    );

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let server = axum::Server::bind(&addr).serve(app.into_make_service());
    let task = tokio::spawn(async move {
        tokio::select! {
            _ = server => {},
            _ = rx => {},
        }
    });

    Ok(MetricsServerHandle {
        shutdown: Some(tx),
        task,
    })
}

pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();
}
