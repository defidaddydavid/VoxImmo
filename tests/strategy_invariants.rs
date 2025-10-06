use polymarket_sweeper::exchange::types::Side;
use polymarket_sweeper::{
    config::AppConfig,
    strategy::{
        fair_value::estimate_threshold_market,
        mispricing::{detect, MispricingAction},
        position::Position,
        risk::RiskManager,
    },
};
use proptest::prelude::*;

#[test]
fn fair_value_behaves_monotonically() {
    let near = estimate_threshold_market(40_000.0, 39_500.0, 0.05, Some(0.5), 5.0);
    let far = estimate_threshold_market(40_000.0, 45_000.0, 0.05, Some(0.5), 5.0);
    assert!(near.probability > far.probability);
}

#[test]
fn mispricing_only_triggers_outside_threshold() {
    let est = estimate_threshold_market(40_000.0, 40_000.0, 0.05, Some(0.5), 5.0);
    let sig = detect(0.51, est, 0.05);
    assert_eq!(sig.action, MispricingAction::Flat);
}

#[test]
fn risk_caps_block_excess() {
    let cfg = AppConfig::default();
    let manager = RiskManager::new(cfg.risk.clone(), 10_000.0);
    assert!(manager.can_enter(100.0, 500.0, 9_500.0, 3.0));
    assert!(!manager.can_enter(600.0, 9_000.0, 9_500.0, 0.1));
}

#[test]
fn fifo_reconciles() {
    let mut pos = Position::default();
    pos.apply_fill(Side::Buy, 10.0, 40);
    pos.apply_fill(Side::Buy, 5.0, 60);
    pos.apply_fill(Side::Sell, 8.0, 80);
    assert!(pos.realized_pnl_cents > 0);
}

proptest! {
    #[test]
    fn inventory_stays_finite(prices in prop::collection::vec(10..90i32, 1..20)) {
        let mut pos = Position::default();
        for price in prices {
            pos.apply_fill(Side::Buy, 1.0, price);
            pos.apply_fill(Side::Sell, 0.5, price + 1);
        }
        prop_assert!(pos.net_qty().abs() < 1000.0);
        prop_assert!(pos.realized_pnl_cents.abs() < 1_000_000);
    }
}
