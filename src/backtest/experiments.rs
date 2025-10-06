use crate::config::AppConfig;

#[derive(Debug, Clone)]
pub struct ExperimentConfig {
    pub deviation_threshold: f64,
    pub clip_usd: f64,
    pub spread_bps: f64,
    pub exit_horizon_minutes: u64,
}

pub fn default_experiments() -> Vec<ExperimentConfig> {
    let thresholds = [0.03, 0.05, 0.07];
    let clips = [25.0, 50.0, 75.0];
    let spreads = [40.0, 60.0, 80.0];
    let horizons = [20, 40, 60];

    let mut experiments = Vec::new();
    for &thr in &thresholds {
        for &clip in &clips {
            for &spread in &spreads {
                experiments.push(ExperimentConfig {
                    deviation_threshold: thr,
                    clip_usd: clip,
                    spread_bps: spread,
                    exit_horizon_minutes: horizons[1],
                });
            }
        }
    }
    for &h in &horizons {
        experiments.push(ExperimentConfig {
            deviation_threshold: 0.05,
            clip_usd: 50.0,
            spread_bps: 60.0,
            exit_horizon_minutes: h,
        });
    }
    experiments
}

pub fn apply_experiment(base: &AppConfig, exp: &ExperimentConfig) -> AppConfig {
    let mut cfg = base.clone();
    cfg.mispricing.deviation_threshold = exp.deviation_threshold;
    cfg.quoting.clip_usd = exp.clip_usd;
    cfg.quoting.target_spread_bps = exp.spread_bps;
    cfg.sim.exit_horizon_minutes = exp.exit_horizon_minutes;
    cfg
}
