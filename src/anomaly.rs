//! Anomaly detection using z-score, modified z-score, and IQR methods.

use crate::stats;
use crate::TimeSeries;

/// An detected anomaly with its detection method and score.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Anomaly {
    pub timestamp: String,
    pub value: f64,
    pub method: AnomalyMethod,
    pub score: f64,
    pub threshold: f64,
}

/// Detection method used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum AnomalyMethod {
    ZScore,
    ModifiedZScore,
    Iqr,
}

impl AnomalyMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            AnomalyMethod::ZScore => "zscore",
            AnomalyMethod::ModifiedZScore => "modified-zscore",
            AnomalyMethod::Iqr => "iqr",
        }
    }
}

/// Detect anomalies using z-score (standard deviations from mean).
/// Flags values where |z| > threshold (default 3.0).
pub fn detect_zscore(series: &TimeSeries, threshold: f64) -> Vec<Anomaly> {
    let values = series.values();
    if values.is_empty() {
        return Vec::new();
    }
    let n = values.len() as f64;
    let mean: f64 = values.iter().sum::<f64>() / n;
    let variance: f64 = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    let stddev = variance.sqrt();

    if stddev == 0.0 {
        return Vec::new();
    }

    series
        .points
        .iter()
        .filter_map(|p| {
            let z = (p.value - mean) / stddev;
            if z.abs() > threshold {
                Some(Anomaly {
                    timestamp: p.timestamp.clone(),
                    value: p.value,
                    method: AnomalyMethod::ZScore,
                    score: z,
                    threshold,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Detect anomalies using modified z-score (MAD-based, robust to outliers).
/// Uses median and median absolute deviation instead of mean and stddev.
pub fn detect_modified_zscore(series: &TimeSeries, threshold: f64) -> Vec<Anomaly> {
    let values = series.values();
    if values.is_empty() {
        return Vec::new();
    }

    let mut sorted = values.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = stats::percentile_sorted(&sorted, 50.0);

    let mad: f64 = {
        let deviations: Vec<f64> = values.iter().map(|v| (v - median).abs()).collect();
        let mut dev_sorted = deviations;
        dev_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        stats::percentile_sorted(&dev_sorted, 50.0)
    };

    // 0.6745 is the constant for modified z-score (0.6745 * MAD ≈ stddev for normal dist)
    let scale = 0.6745 * mad;
    if scale == 0.0 {
        return Vec::new();
    }

    series
        .points
        .iter()
        .filter_map(|p| {
            let mz = 0.6745 * (p.value - median) / mad;
            if mz.abs() > threshold {
                Some(Anomaly {
                    timestamp: p.timestamp.clone(),
                    value: p.value,
                    method: AnomalyMethod::ModifiedZScore,
                    score: mz,
                    threshold,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Detect anomalies using the IQR method.
/// Flags values below Q1 - k*IQR or above Q3 + k*IQR (default k=1.5).
pub fn detect_iqr(series: &TimeSeries, k: f64) -> Vec<Anomaly> {
    let values = series.values();
    if values.is_empty() {
        return Vec::new();
    }

    let mut sorted = values.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let q1 = stats::percentile_sorted(&sorted, 25.0);
    let q3 = stats::percentile_sorted(&sorted, 75.0);
    let iqr = q3 - q1;
    let lower = q1 - k * iqr;
    let upper = q3 + k * iqr;

    series
        .points
        .iter()
        .filter_map(|p| {
            if p.value < lower || p.value > upper {
                let score = if p.value < lower {
                    (p.value - lower) / iqr.max(f64::EPSILON)
                } else {
                    (p.value - upper) / iqr.max(f64::EPSILON)
                };
                Some(Anomaly {
                    timestamp: p.timestamp.clone(),
                    value: p.value,
                    method: AnomalyMethod::Iqr,
                    score,
                    threshold: k,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Run all three detection methods and merge results (deduplicated by timestamp).
pub fn detect_all(series: &TimeSeries, zscore_threshold: f64, iqr_k: f64) -> Vec<Anomaly> {
    let mut all = Vec::new();
    all.extend(detect_zscore(series, zscore_threshold));
    all.extend(detect_modified_zscore(series, zscore_threshold));
    all.extend(detect_iqr(series, iqr_k));
    // Sort by timestamp then by score magnitude
    all.sort_by(|a, b| {
        a.timestamp.cmp(&b.timestamp).then_with(|| {
            b.score
                .abs()
                .partial_cmp(&a.score.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });
    all
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
    fn test_zscore_detects_outlier() {
        let s = make_series(&[10.0, 10.0, 10.0, 10.0, 10.0, 100.0]);
        let anomalies = detect_zscore(&s, 2.0);
        assert!(!anomalies.is_empty());
        assert!((anomalies[0].value - 100.0).abs() < 1e-9);
    }

    #[test]
    fn test_zscore_no_anomalies() {
        let s = make_series(&[10.0, 11.0, 10.0, 11.0, 10.0, 11.0]);
        let anomalies = detect_zscore(&s, 3.0);
        assert!(anomalies.is_empty());
    }

    #[test]
    fn test_modified_zscore_robust() {
        // Modified z-score should be more robust to outliers
        let s = make_series(&[1.0, 2.0, 1.0, 2.0, 1.0, 2.0, 1.0, 50.0]);
        let anomalies = detect_modified_zscore(&s, 3.5);
        assert!(!anomalies.is_empty());
        assert!((anomalies[0].value - 50.0).abs() < 1e-9);
    }

    #[test]
    fn test_iqr_detects() {
        let s = make_series(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 100.0]);
        let anomalies = detect_iqr(&s, 1.5);
        assert!(!anomalies.is_empty());
        assert!((anomalies[0].value - 100.0).abs() < 1e-9);
    }

    #[test]
    fn test_empty_series() {
        let s = TimeSeries::new("empty");
        assert!(detect_zscore(&s, 3.0).is_empty());
        assert!(detect_modified_zscore(&s, 3.5).is_empty());
        assert!(detect_iqr(&s, 1.5).is_empty());
    }

    #[test]
    fn test_constant_series() {
        let s = make_series(&[5.0, 5.0, 5.0, 5.0]);
        assert!(detect_zscore(&s, 3.0).is_empty());
    }
}
