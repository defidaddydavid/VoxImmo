use super::fair_value::FairValueEstimate;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MispricingAction {
    BuyYes,
    SellYes,
    Flat,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MispricingSignal {
    pub action: MispricingAction,
    pub strength: f64,
}

impl MispricingSignal {
    pub fn flat() -> Self {
        Self {
            action: MispricingAction::Flat,
            strength: 0.0,
        }
    }
}

pub fn detect(
    mid_probability: f64,
    estimate: FairValueEstimate,
    threshold: f64,
) -> MispricingSignal {
    let diff = mid_probability - estimate.probability;
    if diff.abs() < threshold {
        MispricingSignal::flat()
    } else if diff > 0.0 {
        MispricingSignal {
            action: MispricingAction::SellYes,
            strength: (diff / threshold).max(0.0),
        }
    } else {
        MispricingSignal {
            action: MispricingAction::BuyYes,
            strength: (-diff / threshold).max(0.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_signal_within_threshold() {
        let est = FairValueEstimate::flat(0.5);
        assert_eq!(detect(0.52, est, 0.05).action, MispricingAction::Flat);
    }

    #[test]
    fn buy_signal_when_cheap() {
        let est = FairValueEstimate::flat(0.6);
        let sig = detect(0.4, est, 0.05);
        assert_eq!(sig.action, MispricingAction::BuyYes);
        assert!(sig.strength > 1.0);
    }
}
