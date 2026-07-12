//! Seasonality detection via autocorrelation function (ACF).

use crate::TimeSeries;

/// Seasonality detection result.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SeasonalityResult {
    /// Autocorrelation values for lags 1..=max_lag.
    pub autocorrelations: Vec<f64>,
    /// Detected seasonal period (lag with highest ACF), if any.
    pub period: Option<usize>,
    /// Strength of the seasonality (0.0 to 1.0).
    pub strength: f64,
    /// Lag with the maximum autocorrelation.
    pub max_lag: usize,
    /// Maximum autocorrelation value.
    pub max_acf: f64,
}

/// Compute the autocorrelation function for lags 0..=max_lag.
pub fn autocorrelation(values: &[f64], max_lag: usize) -> Vec<f64> {
    let n = values.len();
    if n < 2 {
        return Vec::new();
    }
    let mean: f64 = values.iter().sum::<f64>() / n as f64;
    let variance: f64 = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
    if variance == 0.0 {
        return vec![0.0; max_lag + 1];
    }

    let mut acf = Vec::with_capacity(max_lag + 1);
    for lag in 0..=max_lag {
        if lag >= n {
            acf.push(0.0);
            continue;
        }
        let mut cov = 0.0;
        for i in 0..(n - lag) {
            cov += (values[i] - mean) * (values[i + lag] - mean);
        }
        cov /= n as f64;
        acf.push(cov / variance);
    }
    acf
}

/// Detect seasonality by finding the lag with the highest autocorrelation
/// (excluding lag 0 which is always 1.0).
pub fn detect_seasonality(series: &TimeSeries) -> SeasonalityResult {
    let values = series.values();
    let n = values.len();
    if n < 4 {
        return SeasonalityResult {
            autocorrelations: Vec::new(),
            period: None,
            strength: 0.0,
            max_lag: 0,
            max_acf: 0.0,
        };
    }

    // Use at most n/3 lags to avoid unreliable estimates
    let max_lag = (n / 3).max(2).min(50);
    let acf = autocorrelation(&values, max_lag);

    // Find the lag with the highest ACF (excluding lag 0)
    let mut max_lag = 1;
    let mut max_acf = 0.0;
    for (lag, &val) in acf.iter().enumerate().skip(1) {
        if val > max_acf {
            max_acf = val;
            max_lag = lag;
        }
    }

    // Strength: ACF value at the detected period (0.0 to 1.0)
    let strength = max_acf.max(0.0).min(1.0);

    // Only report a period if the ACF is above a threshold
    let period = if strength > 0.3 { Some(max_lag) } else { None };

    SeasonalityResult {
        autocorrelations: acf,
        period,
        strength,
        max_lag,
        max_acf,
    }
}

/// Compute partial autocorrelation function (PACF) using the Yule-Walker method.
pub fn partial_autocorrelation(values: &[f64], max_lag: usize) -> Vec<f64> {
    let acf = autocorrelation(values, max_lag);
    if acf.is_empty() {
        return Vec::new();
    }
    let n = acf.len();
    let mut pacf = vec![0.0; n];
    pacf[0] = 1.0;

    // Durbin-Levinson recursion
    for k in 1..n {
        let mut numerator = acf[k];
        for j in 0..(k - 1) {
            numerator -= pacf[k - 1] * acf[k - 1 - j];
        }
        let mut denominator = 1.0;
        for j in 0..(k - 1) {
            denominator -= pacf[j] * acf[k - 1 - j];
        }
        if denominator.abs() < f64::EPSILON {
            pacf[k] = 0.0;
        } else {
            pacf[k] = numerator / denominator;
        }
    }
    pacf
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_series(vals: &[f64]) -> TimeSeries {
        let pairs: Vec<(String, f64)> = vals
            .iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), *v))
            .collect();
        TimeSeries::from_pairs("test", pairs)
    }

    #[test]
    fn test_autocorrelation_lag0() {
        let vals = [1.0, 2.0, 3.0, 4.0, 5.0];
        let acf = autocorrelation(&vals, 3);
        assert!((acf[0] - 1.0).abs() < 1e-9); // lag 0 is always 1.0
    }

    #[test]
    fn test_seasonality_periodic() {
        // Create a sine wave with period 4
        let mut vals = Vec::new();
        for i in 0..40 {
            vals.push((2.0 * std::f64::consts::PI * i as f64 / 4.0).sin());
        }
        let s = make_series(&vals);
        let result = detect_seasonality(&s);
        assert!(result.period.is_some());
        assert_eq!(result.period.unwrap(), 4);
        assert!(result.strength > 0.3);
    }

    #[test]
    fn test_seasonality_random() {
        // Non-periodic data should not detect strong seasonality
        let vals = [1.0, 5.0, 2.0, 8.0, 3.0, 7.0, 1.0, 9.0, 4.0, 6.0];
        let s = make_series(&vals);
        let result = detect_seasonality(&s);
        // Random data should have low strength
        assert!(result.strength < 0.8);
    }

    #[test]
    fn test_short_series() {
        let s = make_series(&[1.0, 2.0]);
        let result = detect_seasonality(&s);
        assert!(result.period.is_none());
    }

    #[test]
    fn test_constant_series() {
        let s = make_series(&[5.0, 5.0, 5.0, 5.0, 5.0]);
        let acf = autocorrelation(&s.values(), 3);
        assert!(acf.iter().all(|v| v.abs() < 1e-9));
    }

    #[test]
    fn test_pacf_basic() {
        let vals = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let pacf = partial_autocorrelation(&vals, 4);
        assert_eq!(pacf.len(), 5);
        assert!((pacf[0] - 1.0).abs() < 1e-9);
    }
}
