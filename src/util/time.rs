use chrono::{DateTime, Utc};

pub fn time_to_expiry_years(now: DateTime<Utc>, expiry: DateTime<Utc>) -> f64 {
    let secs = (expiry - now).num_seconds().max(0) as f64;
    secs / (365.0 * 24.0 * 3600.0)
}
