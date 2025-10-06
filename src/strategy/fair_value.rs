use crate::util::math::{clamp01, logistic};
use statrs::distribution::{ContinuousCDF, Normal};

#[derive(Debug, Clone)]
pub struct FairValueInputs {
    pub spot: f64,
    pub threshold: f64,
    pub time_to_expiry_years: f64,
    pub implied_volatility: Option<f64>,
    pub historical_prices: Vec<f64>,
    pub data_age_seconds: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FairValueEstimate {
    pub probability: f64,
    pub confidence: f64,
}

impl FairValueEstimate {
    pub fn flat(probability: f64) -> Self {
        Self {
            probability: clamp01(probability),
            confidence: 0.5,
        }
    }
}

pub fn estimate_threshold_market(
    spot: f64,
    threshold: f64,
    time_to_expiry_years: f64,
    volatility: Option<f64>,
    data_age_seconds: f64,
) -> FairValueEstimate {
    let inputs = FairValueInputs {
        spot,
        threshold,
        time_to_expiry_years,
        implied_volatility: volatility,
        historical_prices: Vec::new(),
        data_age_seconds,
    };
    estimate_with_inputs(&inputs)
}

pub fn estimate_with_inputs(inputs: &FairValueInputs) -> FairValueEstimate {
    if inputs.spot <= 0.0 || inputs.threshold <= 0.0 || inputs.time_to_expiry_years <= 0.0 {
        return FairValueEstimate {
            probability: 0.5,
            confidence: 0.1,
        };
    }

    let recency_confidence = (1.0 / (1.0 + inputs.data_age_seconds / 60.0)).clamp(0.0, 1.0);
    let realized_vol = realized_volatility(&inputs.historical_prices);
    let volatility = blend_volatility(inputs.implied_volatility, realized_vol);

    let probability = if let Some(vol) = volatility.filter(|v| *v > 0.0) {
        let mu = drift_estimate(&inputs.historical_prices);
        let sigma = vol.max(1e-4);
        let denom = sigma * inputs.time_to_expiry_years.sqrt();
        if denom <= f64::EPSILON {
            (inputs.spot / (inputs.spot + inputs.threshold)).clamp(0.0, 1.0)
        } else {
            let normal = Normal::new(0.0, 1.0).expect("normal");
            let log_ratio = (inputs.threshold / inputs.spot).ln();
            let d = (log_ratio - (mu - 0.5 * sigma * sigma) * inputs.time_to_expiry_years) / denom;
            let base = 1.0 - normal.cdf(d);
            apply_skew_adjustment(base, inputs.spot, inputs.threshold, sigma)
        }
    } else {
        let rel = (inputs.spot - inputs.threshold) / inputs.threshold;
        logistic(rel * 12.0 * inputs.time_to_expiry_years.clamp(0.1, 2.0))
    };

    let confidence = (recency_confidence * 0.6)
        + (volatility.map(|_| 0.3).unwrap_or(0.0))
        + (realized_vol.map(|_| 0.1).unwrap_or(0.0));

    FairValueEstimate {
        probability: clamp01(probability),
        confidence: clamp01(confidence),
    }
}

fn realized_volatility(prices: &[f64]) -> Option<f64> {
    if prices.len() < 3 {
        return None;
    }
    let mut returns = Vec::with_capacity(prices.len() - 1);
    for window in prices.windows(2) {
        if window[0] <= 0.0 || window[1] <= 0.0 {
            continue;
        }
        returns.push((window[1] / window[0]).ln());
    }
    if returns.len() < 2 {
        return None;
    }
    let mean = returns.iter().copied().sum::<f64>() / returns.len() as f64;
    let variance = returns
        .iter()
        .map(|r| {
            let diff = r - mean;
            diff * diff
        })
        .sum::<f64>()
        / (returns.len() - 1) as f64;
    Some(variance.max(0.0).sqrt() * (returns.len() as f64).sqrt())
}

fn blend_volatility(implied: Option<f64>, realized: Option<f64>) -> Option<f64> {
    match (implied, realized) {
        (Some(i), Some(r)) => Some(0.65 * i + 0.35 * r),
        (Some(i), None) => Some(i),
        (None, Some(r)) => Some(r * 1.1),
        _ => None,
    }
}

fn drift_estimate(prices: &[f64]) -> f64 {
    if prices.len() < 2 {
        return 0.0;
    }
    let start = prices.first().copied().unwrap_or(0.0);
    let end = prices.last().copied().unwrap_or(0.0);
    if start <= 0.0 || end <= 0.0 {
        return 0.0;
    }
    ((end / start).ln()).clamp(-0.5, 0.5)
}

fn apply_skew_adjustment(base: f64, spot: f64, threshold: f64, volatility: f64) -> f64 {
    let moneyness = (spot - threshold) / threshold;
    let skew = (moneyness * 3.0) / (1.0 + volatility * 2.0);
    clamp01(base + skew)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn itm_probability_high() {
        let est = estimate_threshold_market(40_000.0, 30_000.0, 0.1, Some(0.6), 10.0);
        assert!(est.probability > 0.7);
    }

    #[test]
    fn otm_probability_low() {
        let est = estimate_threshold_market(20_000.0, 50_000.0, 0.1, Some(0.6), 10.0);
        assert!(est.probability < 0.4);
    }

    #[test]
    fn realized_vol_improves_confidence() {
        let inputs = FairValueInputs {
            spot: 42_000.0,
            threshold: 40_000.0,
            time_to_expiry_years: 0.2,
            implied_volatility: Some(0.5),
            historical_prices: vec![40_000.0, 41_000.0, 42_500.0, 44_000.0],
            data_age_seconds: 5.0,
        };
        let est = estimate_with_inputs(&inputs);
        assert!(est.confidence > 0.5);
    }
}
