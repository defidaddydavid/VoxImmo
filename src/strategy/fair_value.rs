use crate::util::math::{clamp01, logistic};
use statrs::distribution::{ContinuousCDF, Normal};

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
    let confidence = (1.0 / (1.0 + data_age_seconds / 60.0)).clamp(0.0, 1.0);
    if spot <= 0.0 || threshold <= 0.0 || time_to_expiry_years <= 0.0 {
        return FairValueEstimate {
            probability: 0.5,
            confidence: confidence * 0.5,
        };
    }

    let probability = match volatility.filter(|v| *v > 0.0) {
        Some(vol) => {
            let mu = 0.0_f64;
            let sigma = vol;
            let denom = sigma * time_to_expiry_years.sqrt();
            if denom <= f64::EPSILON {
                (spot / (spot + threshold)).clamp(0.0, 1.0)
            } else {
                let normal = Normal::new(0.0, 1.0).expect("normal");
                let d = ((threshold / spot).ln()
                    - (mu - 0.5 * sigma * sigma) * time_to_expiry_years)
                    / denom;
                1.0 - normal.cdf(d)
            }
        }
        None => {
            let rel = (spot - threshold) / threshold;
            logistic(rel * 10.0 * time_to_expiry_years.clamp(0.1, 2.0))
        }
    };

    FairValueEstimate {
        probability: clamp01(probability),
        confidence,
    }
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
}
