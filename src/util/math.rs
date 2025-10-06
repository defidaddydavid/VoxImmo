pub fn logistic(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

pub fn clamp01(x: f64) -> f64 {
    if x < 0.0 {
        0.0
    } else if x > 1.0 {
        1.0
    } else {
        x
    }
}

pub fn bps_to_fraction(bps: f64) -> f64 {
    bps / 10_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logistic_midpoint() {
        assert!((logistic(0.0) - 0.5).abs() < 1e-6);
    }
}
