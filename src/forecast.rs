//! Forecasting: simple moving average, exponential smoothing, and Holt's linear method.

use crate::TimeSeries;

/// A forecast result with predicted values and optional confidence bounds.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Forecast {
    /// Forecasted values for future periods.
    pub predictions: Vec<f64>,
    /// The method used.
    pub method: String,
    /// Mean absolute error of the fit on historical data.
    pub mae: f64,
    /// Root mean squared error of the fit on historical data.
    pub rmse: f64,
}

/// Simple moving average forecast.
/// Predicts the next `horizon` values as the average of the last `window` values.
pub fn forecast_sma(series: &TimeSeries, window: usize, horizon: usize) -> Forecast {
    let values = series.values();
    if values.is_empty() || window == 0 {
        return Forecast {
            predictions: Vec::new(),
            method: "simple-moving-average".into(),
            mae: 0.0,
            rmse: 0.0,
        };
    }

    let w = window.min(values.len());
    let last_window: Vec<f64> = values[values.len() - w..].to_vec();
    let avg = last_window.iter().sum::<f64>() / w as f64;

    // Compute in-sample fit error
    let fitted: Vec<f64> = crate::stats::moving_average(&values, w);
    let errors: Vec<f64> = values
        .iter()
        .zip(fitted.iter())
        .map(|(actual, pred)| actual - pred)
        .collect();
    let (mae, rmse) = compute_errors(&errors);

    Forecast {
        predictions: vec![avg; horizon],
        method: format!("simple-moving-average(w={w})"),
        mae,
        rmse,
    }
}

/// Simple exponential smoothing (SES) forecast.
/// `alpha` is the smoothing factor (0 < alpha < 1).
pub fn forecast_ses(series: &TimeSeries, alpha: f64, horizon: usize) -> Forecast {
    let values = series.values();
    if values.is_empty() {
        return Forecast {
            predictions: Vec::new(),
            method: "exponential-smoothing".into(),
            mae: 0.0,
            rmse: 0.0,
        };
    }

    let a = alpha.clamp(0.001, 0.999);
    let mut level = values[0];
    let mut fitted = Vec::with_capacity(values.len());
    fitted.push(level);

    for &v in &values[1..] {
        let prev_level = level;
        level = a * v + (1.0 - a) * prev_level;
        fitted.push(level);
    }

    let errors: Vec<f64> = values
        .iter()
        .zip(fitted.iter())
        .map(|(actual, pred)| actual - pred)
        .collect();
    let (mae, rmse) = compute_errors(&errors);

    Forecast {
        predictions: vec![level; horizon],
        method: format!("exponential-smoothing(alpha={a:.3})"),
        mae,
        rmse,
    }
}

/// Holt's linear trend method (double exponential smoothing).
/// `alpha` is level smoothing, `beta` is trend smoothing.
pub fn forecast_holt(series: &TimeSeries, alpha: f64, beta: f64, horizon: usize) -> Forecast {
    let values = series.values();
    if values.len() < 2 {
        return Forecast {
            predictions: Vec::new(),
            method: "holt-linear".into(),
            mae: 0.0,
            rmse: 0.0,
        };
    }

    let a = alpha.clamp(0.001, 0.999);
    let b = beta.clamp(0.001, 0.999);

    // Initialize: level = first value, trend = second - first
    let mut level = values[0];
    let mut trend = values[1] - values[0];
    let mut fitted = Vec::with_capacity(values.len());
    fitted.push(level);

    for &v in &values[1..] {
        let prev_level = level;
        let prev_trend = trend;
        level = a * v + (1.0 - a) * (prev_level + prev_trend);
        trend = b * (level - prev_level) + (1.0 - b) * prev_trend;
        fitted.push(level + trend);
    }

    let errors: Vec<f64> = values
        .iter()
        .zip(fitted.iter())
        .map(|(actual, pred)| actual - pred)
        .collect();
    let (mae, rmse) = compute_errors(&errors);

    // Forecast: level + h * trend
    let predictions: Vec<f64> = (1..=horizon).map(|h| level + h as f64 * trend).collect();

    Forecast {
        predictions,
        method: format!("holt-linear(alpha={a:.3},beta={b:.3})"),
        mae,
        rmse,
    }
}

/// Naive forecast: predicts the last value for all future periods.
pub fn forecast_naive(series: &TimeSeries, horizon: usize) -> Forecast {
    let values = series.values();
    if values.is_empty() {
        return Forecast {
            predictions: Vec::new(),
            method: "naive".into(),
            mae: 0.0,
            rmse: 0.0,
        };
    }

    let last = values[values.len() - 1];
    // In-sample fit: each prediction is the previous value
    let mut errors = Vec::new();
    for i in 1..values.len() {
        errors.push(values[i] - values[i - 1]);
    }
    let (mae, rmse) = compute_errors(&errors);

    Forecast {
        predictions: vec![last; horizon],
        method: "naive".into(),
        mae,
        rmse,
    }
}

/// Seasonal naive forecast: predicts using values from the last full season.
pub fn forecast_seasonal_naive(series: &TimeSeries, period: usize, horizon: usize) -> Forecast {
    let values = series.values();
    if values.is_empty() || period == 0 {
        return Forecast {
            predictions: Vec::new(),
            method: "seasonal-naive".into(),
            mae: 0.0,
            rmse: 0.0,
        };
    }

    let p = period.min(values.len());
    let predictions: Vec<f64> = (0..horizon)
        .map(|h| {
            let idx = values.len() - p + (h % p);
            values.get(idx).copied().unwrap_or(0.0)
        })
        .collect();

    // In-sample fit: use value from one period ago
    let mut errors = Vec::new();
    for i in p..values.len() {
        errors.push(values[i] - values[i - p]);
    }
    let (mae, rmse) = compute_errors(&errors);

    Forecast {
        predictions,
        method: format!("seasonal-naive(period={p})"),
        mae,
        rmse,
    }
}

fn compute_errors(errors: &[f64]) -> (f64, f64) {
    if errors.is_empty() {
        return (0.0, 0.0);
    }
    let n = errors.len() as f64;
    let mae = errors.iter().map(|e| e.abs()).sum::<f64>() / n;
    let rmse = (errors.iter().map(|e| e.powi(2)).sum::<f64>() / n).sqrt();
    (mae, rmse)
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
    fn test_sma_forecast() {
        let s = make_series(&[10.0, 20.0, 30.0, 40.0, 50.0]);
        let f = forecast_sma(&s, 3, 5);
        assert_eq!(f.predictions.len(), 5);
        // Last 3 values: 30, 40, 50 → avg = 40
        assert!((f.predictions[0] - 40.0).abs() < 1e-9);
    }

    #[test]
    fn test_ses_forecast() {
        let s = make_series(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let f = forecast_ses(&s, 0.5, 3);
        assert_eq!(f.predictions.len(), 3);
        // All predictions should be the same (constant forecast)
        assert!(f
            .predictions
            .iter()
            .all(|p| (p - f.predictions[0]).abs() < 1e-9));
    }

    #[test]
    fn test_holt_trend() {
        let s = make_series(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
        let f = forecast_holt(&s, 0.5, 0.5, 3);
        assert_eq!(f.predictions.len(), 3);
        // Should predict increasing values (upward trend)
        assert!(f.predictions[1] > f.predictions[0]);
        assert!(f.predictions[2] > f.predictions[1]);
    }

    #[test]
    fn test_naive_forecast() {
        let s = make_series(&[1.0, 2.0, 3.0, 42.0]);
        let f = forecast_naive(&s, 3);
        assert_eq!(f.predictions, [42.0, 42.0, 42.0]);
    }

    #[test]
    fn test_seasonal_naive() {
        let s = make_series(&[1.0, 2.0, 3.0, 1.0, 2.0, 3.0, 1.0, 2.0, 3.0]);
        let f = forecast_seasonal_naive(&s, 3, 4);
        assert_eq!(f.predictions.len(), 4);
        // Should repeat the last season: 1, 2, 3, 1
        assert!((f.predictions[0] - 1.0).abs() < 1e-9);
        assert!((f.predictions[1] - 2.0).abs() < 1e-9);
        assert!((f.predictions[2] - 3.0).abs() < 1e-9);
        assert!((f.predictions[3] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_empty_series() {
        let s = TimeSeries::new("empty");
        let f = forecast_sma(&s, 3, 5);
        assert!(f.predictions.is_empty());
    }

    #[test]
    fn test_error_metrics() {
        let s = make_series(&[10.0, 20.0, 30.0]);
        let f = forecast_naive(&s, 1);
        assert!(f.mae >= 0.0);
        assert!(f.rmse >= 0.0);
    }
}
