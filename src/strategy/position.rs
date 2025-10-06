use crate::exchange::types::Side;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
struct Lot {
    qty: f64,
    price_cents: i32,
}

#[derive(Debug, Clone)]
pub struct Position {
    lots: VecDeque<Lot>,
    pub realized_pnl_cents: i64,
}

impl Default for Position {
    fn default() -> Self {
        Self {
            lots: VecDeque::new(),
            realized_pnl_cents: 0,
        }
    }
}

impl Position {
    pub fn net_qty(&self) -> f64 {
        self.lots.iter().map(|lot| lot.qty).sum()
    }

    pub fn avg_cost_cents(&self) -> Option<f64> {
        let total_qty: f64 = self.net_qty();
        if total_qty.abs() < f64::EPSILON {
            None
        } else {
            let total_cost: f64 = self
                .lots
                .iter()
                .map(|lot| lot.qty * lot.price_cents as f64)
                .sum();
            Some(total_cost / total_qty)
        }
    }

    pub fn apply_fill(&mut self, side: Side, qty: f64, price_cents: i32) {
        match side {
            Side::Buy => self.buy(qty, price_cents),
            Side::Sell => self.sell(qty, price_cents),
        }
    }

    fn buy(&mut self, qty: f64, price_cents: i32) {
        if qty <= 0.0 {
            return;
        }
        self.lots.push_back(Lot { qty, price_cents });
    }

    fn sell(&mut self, mut qty: f64, price_cents: i32) {
        if qty <= 0.0 {
            return;
        }
        while qty > 0.0 {
            if let Some(mut lot) = self.lots.pop_front() {
                let take = qty.min(lot.qty);
                let pnl = (price_cents - lot.price_cents) as f64 * take;
                self.realized_pnl_cents += pnl.round() as i64;
                lot.qty -= take;
                qty -= take;
                if lot.qty > f64::EPSILON {
                    self.lots.push_front(lot);
                    break;
                }
            } else {
                // short sell: treat as negative lot
                self.lots.push_front(Lot {
                    qty: -qty,
                    price_cents,
                });
                break;
            }
        }
    }

    pub fn unrealized_pnl_cents(&self, mark_price_cents: i32) -> i64 {
        self.lots
            .iter()
            .map(|lot| (mark_price_cents - lot.price_cents) as f64 * lot.qty)
            .sum::<f64>()
            .round() as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifo_realized_pnl() {
        let mut pos = Position::default();
        pos.apply_fill(Side::Buy, 10.0, 40);
        pos.apply_fill(Side::Buy, 5.0, 50);
        pos.apply_fill(Side::Sell, 8.0, 60);
        assert_eq!(pos.realized_pnl_cents, (60 - 40) as i64 * 8);
        assert!(pos.net_qty() > 0.0);
    }
}
