//! Trend analysis via linear regression and decomposition.

use crate::TimeSeries;

/// Linear regression result for trend analysis.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TrendResult {
    /// Slope of the linear trend.
    pub slope: f64,
    /// Y-intercept of the linear trend.
    pub intercept: f64,
    /// R-squared coefficient of determination (0.0 to 1.0).
    pub r_squared: f64,
    /// Pearson correlation coefficient (-1.0 to 1.0).
    pub correlation: f64,
    /// Whether the trend is increasing, decreasing, or flat.
    pub direction: TrendDirection,
    /// Estimated annualized growth rate (if timestamps are numeric).
    pub growth_rate: Option<f64>,
}

/// Direction of the trend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Flat,
}

impl TrendDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            TrendDirection::Increasing => "increasing",
            TrendDirection::Decreasing => "decreasing",
            TrendDirection::Flat => "flat",
        }
    }
}

/// Fit a linear regression (ordinary least squares) to the time series.
/// X values are the index (0, 1, 2, ...) or parsed numeric timestamps.
pub fn analyze_trend(series: &TimeSeries) -> Option<TrendResult> {
    let values = series.values();
    if values.len() < 2 {
        return None;
    }

    // Try numeric timestamps, fall back to indices
    let x_values: Vec<f64> = if series
        .points
        .iter()
        .all(|p| p.timestamp.parse::<f64>().is_ok())
    {
        series
            .points
            .iter()
            .map(|p| p.timestamp.parse::<f64>().unwrap())
            .collect()
    } else {
        (0..values.len()).map(|i| i as f64).collect()
    };

    let n = values.len() as f64;
    let sum_x: f64 = x_values.iter().sum();
    let sum_y: f64 = values.iter().sum();
    let sum_xy: f64 = x_values.iter().zip(values.iter()).map(|(x, y)| x * y).sum();
    let sum_x2: f64 = x_values.iter().map(|x| x * x).sum();
    let sum_y2: f64 = values.iter().map(|y| y * y).sum();

    let denom = n * sum_x2 - sum_x * sum_x;
    if denom.abs() < f64::EPSILON {
        return None;
    }

    let slope = (n * sum_xy - sum_x * sum_y) / denom;
    let intercept = (sum_y - slope * sum_x) / n;

    // Pearson correlation
    let var_x = (n * sum_x2 - sum_x * sum_x) / n;
    let var_y = (n * sum_y2 - sum_y * sum_y) / n;
    let cov_xy = (n * sum_xy - sum_x * sum_y) / n;
    let correlation = if var_x > 0.0 && var_y > 0.0 {
        cov_xy / (var_x * var_y).sqrt()
    } else {
        0.0
    };
    let r_squared = correlation * correlation;

    let direction = if slope > 0.0 && slope.abs() > f64::EPSILON {
        TrendDirection::Increasing
    } else if slope < 0.0 && slope.abs() > f64::EPSILON {
        TrendDirection::Decreasing
    } else {
        TrendDirection::Flat
    };

    // Growth rate: slope / mean (percentage per time unit)
    let mean_y = sum_y / n;
    let growth_rate = if mean_y != 0.0 {
        Some((slope / mean_y.abs()) * 100.0)
    } else {
        None
    };

    Some(TrendResult {
        slope,
        intercept,
        r_squared,
        correlation,
        direction,
        growth_rate,
    })
}

/// Decompose a time series into trend and residual components using
/// a centered moving average.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Decomposition {
    /// Trend component (moving average).
    pub trend: Vec<f64>,
    /// Residual (detrended) component.
    pub residual: Vec<f64>,
}

/// Decompose a time series using a centered moving average of the given window.
pub fn decompose(series: &TimeSeries, window: usize) -> Option<Decomposition> {
    let values = series.values();
    if values.len() < window || window < 2 {
        return None;
    }

    let w = if window % 2 == 0 { window + 1 } else { window }; // odd window for centering
    let half = w / 2;
    let n = values.len();

    let mut trend = vec![f64::NAN; n];
    for i in half..(n - half) {
        let window_vals = &values[i - half..=i + half];
        trend[i] = window_vals.iter().sum::<f64>() / w as f64;
    }

    let residual: Vec<f64> = values
        .iter()
        .zip(trend.iter())
        .map(|(v, t)| if t.is_nan() { f64::NAN } else { v - t })
        .collect();

    Some(Decomposition { trend, residual })
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
    fn test_linear_trend_increasing() {
        let s = make_series(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]);
        let trend = analyze_trend(&s).unwrap();
        assert!((trend.slope - 1.0).abs() < 1e-9);
        assert_eq!(trend.direction, TrendDirection::Increasing);
        assert!((trend.r_squared - 1.0).abs() < 1e-6); // perfect linear fit
    }

    #[test]
    fn test_linear_trend_decreasing() {
        let s = make_series(&[10.0, 8.0, 6.0, 4.0, 2.0]);
        let trend = analyze_trend(&s).unwrap();
        assert!(trend.slope < 0.0);
        assert_eq!(trend.direction, TrendDirection::Decreasing);
    }

    #[test]
    fn test_linear_trend_flat() {
        let s = make_series(&[5.0, 5.0, 5.0, 5.0, 5.0]);
        let trend = analyze_trend(&s).unwrap();
        assert!((trend.slope - 0.0).abs() < 1e-9);
        assert_eq!(trend.direction, TrendDirection::Flat);
    }

    #[test]
    fn test_correlation() {
        let s = make_series(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let trend = analyze_trend(&s).unwrap();
        assert!((trend.correlation - 1.0).abs() < 1e-6); // perfect positive correlation
    }

    #[test]
    fn test_growth_rate() {
        let s = make_series(&[100.0, 110.0, 120.0, 130.0]);
        let trend = analyze_trend(&s).unwrap();
        let gr = trend.growth_rate.unwrap();
        // slope=10, mean=115 → growth_rate = 10/115 * 100 ≈ 8.696
        assert!((gr - 8.6957).abs() < 0.01);
    }

    #[test]
    fn test_short_series() {
        let s = make_series(&[1.0]);
        assert!(analyze_trend(&s).is_none());
    }

    #[test]
    fn test_decompose() {
        let s = make_series(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]);
        let decomp = decompose(&s, 3).unwrap();
        assert_eq!(decomp.trend.len(), 7);
        // Edges should be NaN
        assert!(decomp.trend[0].is_nan());
        assert!(decomp.trend[6].is_nan());
        // Center should be the average
        assert!((decomp.trend[3] - 4.0).abs() < 1e-9);
    }
}
