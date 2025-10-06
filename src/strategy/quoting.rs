use crate::{config::QuotingConfig, exchange::types::Side, util::math::bps_to_fraction};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuotePlan {
    pub bid_cents: i32,
    pub ask_cents: i32,
    pub size_shares: f64,
    pub skew_side: Option<Side>,
}

pub fn plan_quotes(
    mid_cents: i32,
    volatility_bps: f64,
    config: &QuotingConfig,
    position_notional: f64,
    risk_capacity_usd: f64,
) -> Option<QuotePlan> {
    if risk_capacity_usd <= 0.0 || mid_cents <= 0 {
        return None;
    }

    let target_half_spread = (config.target_spread_bps.max(volatility_bps)).max(1.0);
    let mid = mid_cents as f64;
    let spread_fraction = bps_to_fraction(target_half_spread);
    let half_spread = (mid * spread_fraction).max(1.0);
    let bid = (mid - half_spread).max(1.0);
    let ask = mid + half_spread;

    let price_dollars = mid / 100.0;
    let clip = (config.clip_usd / price_dollars)
        .min(risk_capacity_usd / price_dollars)
        .max(0.0);
    if clip <= f64::EPSILON {
        return None;
    }

    let skew_side = if position_notional.abs() < f64::EPSILON {
        None
    } else if position_notional > 0.0 {
        Some(Side::Sell)
    } else {
        Some(Side::Buy)
    };

    Some(QuotePlan {
        bid_cents: bid.round() as i32,
        ask_cents: ask.round() as i32,
        size_shares: clip,
        skew_side,
    })
}
