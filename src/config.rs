use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub new_market: NewMarketConfig,
    #[serde(default)]
    pub mispricing: MispricingConfig,
    #[serde(default)]
    pub risk: RiskConfig,
    #[serde(default)]
    pub quoting: QuotingConfig,
    #[serde(default)]
    pub execution: ExecutionConfig,
    #[serde(default)]
    pub sim: SimConfig,
    #[serde(default)]
    pub telemetry: TelemetryConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            new_market: NewMarketConfig::default(),
            mispricing: MispricingConfig::default(),
            risk: RiskConfig::default(),
            quoting: QuotingConfig::default(),
            execution: ExecutionConfig::default(),
            sim: SimConfig::default(),
            telemetry: TelemetryConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn load(path: Option<&Path>) -> Result<Self, config::ConfigError> {
        let mut builder = config::Config::builder()
            .add_source(config::File::from_str(
                include_str!("../configs/default.toml"),
                config::FileFormat::Toml,
            ))
            .set_default("telemetry.prom_port", 9108u16)?;

        if let Some(path) = path {
            if path.exists() {
                builder = builder.add_source(config::File::from(path));
            }
        }

        builder =
            builder.add_source(config::Environment::with_prefix("POLYMARKET").separator("__"));

        builder.build()?.try_deserialize()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewMarketConfig {
    pub window_minutes: u64,
    pub min_liquidity_usd: f64,
}

impl Default for NewMarketConfig {
    fn default() -> Self {
        Self {
            window_minutes: 120,
            min_liquidity_usd: 200.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MispricingConfig {
    pub deviation_threshold: f64,
}

impl Default for MispricingConfig {
    fn default() -> Self {
        Self {
            deviation_threshold: 0.05,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RiskConfig {
    pub max_position_usd_per_market: f64,
    pub max_portfolio_exposure_frac: f64,
    pub drawdown_kill_pct: f64,
    pub resolution_ramp_days: u64,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_position_usd_per_market: 500.0,
            max_portfolio_exposure_frac: 0.10,
            drawdown_kill_pct: 0.15,
            resolution_ramp_days: 7,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuotingConfig {
    pub target_spread_bps: f64,
    pub reprice_move_bps: f64,
    pub clip_usd: f64,
}

impl Default for QuotingConfig {
    fn default() -> Self {
        Self {
            target_spread_bps: 50.0,
            reprice_move_bps: 10.0,
            clip_usd: 50.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExecutionConfig {
    pub latency_target_ms: u64,
    pub cancel_backoff_ms: u64,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            latency_target_ms: 50,
            cancel_backoff_ms: 100,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SimConfig {
    pub fees_bps: f64,
    pub slippage_bps_per_100_usd: f64,
    pub exit_horizon_minutes: u64,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            fees_bps: 20.0,
            slippage_bps_per_100_usd: 3.0,
            exit_horizon_minutes: 60,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelemetryConfig {
    pub prom_port: u16,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self { prom_port: 9108 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_round_trip() {
        let cfg = AppConfig::default();
        assert!(cfg.mispricing.deviation_threshold > 0.0);
        assert!(cfg.risk.max_portfolio_exposure_frac < 1.0);
    }
}
