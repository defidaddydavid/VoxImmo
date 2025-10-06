use crate::{config::SimConfig, exchange::types::Side};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FillOutcome {
    pub filled: f64,
    pub avg_price_cents: f64,
    pub fees_cents: f64,
}

#[derive(Debug, Clone)]
pub struct FillEngine {
    config: SimConfig,
}

impl FillEngine {
    pub fn new(config: SimConfig) -> Self {
        Self { config }
    }

    pub fn sweep(&self, side: Side, size: f64, price_cents: i32) -> FillOutcome {
        let slippage = self.config.slippage_bps_per_100_usd / 10_000.0 * (size / 100.0);
        let price = match side {
            Side::Buy => price_cents as f64 * (1.0 + slippage),
            Side::Sell => price_cents as f64 * (1.0 - slippage),
        };
        let fees = price * size * self.config.fees_bps / 10_000.0 / 100.0;
        FillOutcome {
            filled: size,
            avg_price_cents: price,
            fees_cents: fees,
        }
    }
}
