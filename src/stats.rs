//! Descriptive statistics for time series data.

use crate::TimeSeries;

/// Computed statistics for a series of values.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Stats {
    pub count: usize,
    pub mean: f64,
    pub median: f64,
    pub stddev: f64,
    pub variance: f64,
    pub min: f64,
    pub max: f64,
    pub range: f64,
    pub sum: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
    pub iqr: f64,
    pub cv: f64,
    pub skewness: f64,
    pub kurtosis: f64,
}

/// Compute comprehensive statistics for a time series.
pub fn compute_stats(series: &TimeSeries) -> Option<Stats> {
    let values = series.values();
    if values.is_empty() {
        return None;
    }
    let n = values.len();
    let sum: f64 = values.iter().sum();
    let mean = sum / n as f64;

    let variance = if n > 1 {
        let ssd: f64 = values.iter().map(|v| (v - mean).powi(2)).sum();
        ssd / n as f64
    } else {
        0.0
    };
    let stddev = variance.sqrt();

    let mut sorted = values.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let median = percentile_sorted(&sorted, 50.0);
    let p25 = percentile_sorted(&sorted, 25.0);
    let p75 = percentile_sorted(&sorted, 75.0);
    let p90 = percentile_sorted(&sorted, 90.0);
    let p95 = percentile_sorted(&sorted, 95.0);
    let p99 = percentile_sorted(&sorted, 99.0);
    let iqr = p75 - p25;

    let min = sorted[0];
    let max = sorted[n - 1];
    let range = max - min;

    let cv = if mean != 0.0 {
        stddev / mean.abs()
    } else {
        0.0
    };

    let skewness = if stddev > 0.0 && n > 2 {
        let m3: f64 = values.iter().map(|v| (v - mean).powi(3)).sum::<f64>() / n as f64;
        m3 / stddev.powi(3)
    } else {
        0.0
    };

    let kurtosis = if stddev > 0.0 && n > 3 {
        let m4: f64 = values.iter().map(|v| (v - mean).powi(4)).sum::<f64>() / n as f64;
        (m4 / stddev.powi(4)) - 3.0
    } else {
        0.0
    };

    Some(Stats {
        count: n,
        mean,
        median,
        stddev,
        variance,
        min,
        max,
        range,
        sum,
        p25,
        p50: median,
        p75,
        p90,
        p95,
        p99,
        iqr,
        cv,
        skewness,
        kurtosis,
    })
}

/// Linear interpolation percentile from a sorted slice (R-7 method, same as NumPy).
pub fn percentile_sorted(sorted: &[f64], pct: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let rank = (pct / 100.0) * (sorted.len() - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    if lower == upper {
        return sorted[lower];
    }
    let frac = rank - lower as f64;
    sorted[lower] * (1.0 - frac) + sorted[upper] * frac
}

/// Compute a simple moving average with the given window size.
pub fn moving_average(values: &[f64], window: usize) -> Vec<f64> {
    if values.is_empty() || window == 0 {
        return Vec::new();
    }
    let w = window.min(values.len());
    let mut result = Vec::with_capacity(values.len());
    for i in 0..values.len() {
        let start = i.saturating_sub(w - 1);
        let window_vals = &values[start..=i];
        let avg = window_vals.iter().sum::<f64>() / window_vals.len() as f64;
        result.push(avg);
    }
    result
}

/// Compute the rate of change between consecutive values.
pub fn rate_of_change(values: &[f64]) -> Vec<f64> {
    if values.len() < 2 {
        return Vec::new();
    }
    values.windows(2).map(|w| w[1] - w[0]).collect()
}

/// Compute percentage change between consecutive values.
pub fn pct_change(values: &[f64]) -> Vec<f64> {
    if values.len() < 2 {
        return Vec::new();
    }
    values
        .windows(2)
        .map(|w| {
            if w[0] == 0.0 {
                0.0
            } else {
                (w[1] - w[0]) / w[0].abs() * 100.0
            }
        })
        .collect()
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
    fn test_stats_basic() {
        let s = make_series(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let stats = compute_stats(&s).unwrap();
        assert_eq!(stats.count, 5);
        assert_eq!(stats.mean, 3.0);
        assert_eq!(stats.median, 3.0);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 5.0);
        assert_eq!(stats.sum, 15.0);
    }

    #[test]
    fn test_stats_empty() {
        let s = TimeSeries::new("empty");
        assert!(compute_stats(&s).is_none());
    }

    #[test]
    fn test_percentile() {
        let sorted = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(percentile_sorted(&sorted, 0.0), 1.0);
        assert_eq!(percentile_sorted(&sorted, 50.0), 3.0);
        assert_eq!(percentile_sorted(&sorted, 100.0), 5.0);
    }

    #[test]
    fn test_moving_average() {
        let vals = [1.0, 2.0, 3.0, 4.0, 5.0];
        let ma = moving_average(&vals, 3);
        assert_eq!(ma.len(), 5);
        // First: (1)/1 = 1.0, second: (1+2)/2 = 1.5, third: (1+2+3)/3 = 2.0
        assert!((ma[0] - 1.0).abs() < 1e-9);
        assert!((ma[2] - 2.0).abs() < 1e-9);
        assert!((ma[4] - 4.0).abs() < 1e-9);
    }

    #[test]
    fn test_rate_of_change() {
        let vals = [10.0, 15.0, 12.0, 20.0];
        let roc = rate_of_change(&vals);
        assert_eq!(roc, [5.0, -3.0, 8.0]);
    }

    #[test]
    fn test_pct_change() {
        let vals = [100.0, 110.0, 99.0];
        let pc = pct_change(&vals);
        assert!((pc[0] - 10.0).abs() < 1e-9);
        assert!((pc[1] - (-10.0)).abs() < 1e-9);
    }

    #[test]
    fn test_stddev() {
        let s = make_series(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
        let stats = compute_stats(&s).unwrap();
        // Known: mean=5, variance=4, stddev=2
        assert!((stats.mean - 5.0).abs() < 1e-9);
        assert!((stats.variance - 4.0).abs() < 1e-9);
        assert!((stats.stddev - 2.0).abs() < 1e-9);
    }
}
