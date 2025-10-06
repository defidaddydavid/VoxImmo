use crate::config::RiskConfig;

#[derive(Debug, Clone)]
pub struct RiskManager {
    config: RiskConfig,
    starting_equity_usd: f64,
    pub max_drawdown_observed: f64,
}

impl RiskManager {
    pub fn new(config: RiskConfig, starting_equity_usd: f64) -> Self {
        Self {
            config,
            starting_equity_usd,
            max_drawdown_observed: 0.0,
        }
    }

    pub fn max_notional_for_market(&self, days_to_resolution: f64) -> f64 {
        let ramp_days = self.config.resolution_ramp_days.max(1) as f64;
        let scale = (days_to_resolution / ramp_days).clamp(0.0, 1.0);
        self.config.max_position_usd_per_market * scale
    }

    pub fn can_enter(
        &self,
        market_exposure: f64,
        portfolio_exposure: f64,
        equity_usd: f64,
        days_to_resolution: f64,
    ) -> bool {
        if self.drawdown_pct(equity_usd) >= self.config.drawdown_kill_pct {
            return false;
        }
        if portfolio_exposure > self.config.max_portfolio_exposure_frac * equity_usd {
            return false;
        }
        market_exposure < self.max_notional_for_market(days_to_resolution)
    }

    pub fn record_equity(&mut self, equity_usd: f64) {
        let dd = self.drawdown_pct(equity_usd);
        if dd > self.max_drawdown_observed {
            self.max_drawdown_observed = dd;
        }
    }

    pub fn drawdown_pct(&self, equity_usd: f64) -> f64 {
        if equity_usd >= self.starting_equity_usd {
            0.0
        } else {
            (self.starting_equity_usd - equity_usd) / self.starting_equity_usd
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_when_drawdown_hit() {
        let mut rm = RiskManager::new(RiskConfig::default(), 10_000.0);
        assert!(rm.can_enter(100.0, 500.0, 9_500.0, 3.0));
        rm.record_equity(8_000.0);
        assert!(!rm.can_enter(100.0, 500.0, 8_000.0, 3.0));
    }
}
