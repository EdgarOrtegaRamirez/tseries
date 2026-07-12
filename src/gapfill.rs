//! Gap detection and interpolation for missing data points.

use crate::TimeSeries;

/// A detected gap in the time series.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Gap {
    pub after_timestamp: String,
    pub before_timestamp: String,
    pub gap_size: usize,
}

/// Interpolation method for filling gaps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum FillMethod {
    /// Linear interpolation between neighbors.
    Linear,
    /// Forward-fill (carry last value forward).
    Forward,
    /// Backward-fill (carry next value backward).
    Backward,
    /// Fill with mean of all values.
    Mean,
    /// Fill with zero.
    Zero,
}

impl FillMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            FillMethod::Linear => "linear",
            FillMethod::Forward => "forward",
            FillMethod::Backward => "backward",
            FillMethod::Mean => "mean",
            FillMethod::Zero => "zero",
        }
    }
}

/// Detect gaps where consecutive numeric timestamps differ by more than the
/// median interval. Returns gaps with estimated missing point counts.
pub fn detect_gaps(series: &TimeSeries) -> Vec<Gap> {
    if series.points.len() < 3 {
        return Vec::new();
    }

    // Compute intervals (numeric timestamps only)
    let ts: Vec<f64> = series
        .points
        .iter()
        .filter_map(|p| p.timestamp.parse::<f64>().ok())
        .collect();
    if ts.len() != series.points.len() {
        // Non-numeric timestamps; can't compute intervals
        return Vec::new();
    }

    let intervals: Vec<f64> = ts.windows(2).map(|w| w[1] - w[0]).collect();
    if intervals.is_empty() {
        return Vec::new();
    }

    let mut sorted_intervals = intervals.clone();
    sorted_intervals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = crate::stats::percentile_sorted(&sorted_intervals, 50.0);
    if median <= 0.0 {
        return Vec::new();
    }

    let threshold = median * 2.5;
    let mut gaps = Vec::new();
    for (i, &interval) in intervals.iter().enumerate() {
        if interval > threshold {
            let gap_size = (interval / median).round() as usize - 1;
            if gap_size > 0 {
                gaps.push(Gap {
                    after_timestamp: series.points[i].timestamp.clone(),
                    before_timestamp: series.points[i + 1].timestamp.clone(),
                    gap_size,
                });
            }
        }
    }
    gaps
}

/// Fill NaN values in a series using the specified method.
/// Returns a new series with filled values.
pub fn fill_nans(series: &TimeSeries, method: FillMethod) -> TimeSeries {
    let values: Vec<f64> = series.points.iter().map(|p| p.value).collect();
    let has_nan = values.iter().any(|v| v.is_nan());
    if !has_nan {
        return series.clone();
    }

    let mean: f64 = {
        let valid: Vec<f64> = values.iter().filter(|v| !v.is_nan()).copied().collect();
        if valid.is_empty() {
            0.0
        } else {
            valid.iter().sum::<f64>() / valid.len() as f64
        }
    };

    let mut filled = values.clone();
    match method {
        FillMethod::Zero => {
            for v in &mut filled {
                if v.is_nan() {
                    *v = 0.0;
                }
            }
        }
        FillMethod::Mean => {
            for v in &mut filled {
                if v.is_nan() {
                    *v = mean;
                }
            }
        }
        FillMethod::Forward => {
            let mut last = f64::NAN;
            for v in &mut filled {
                if v.is_nan() {
                    *v = last;
                } else {
                    last = *v;
                }
            }
        }
        FillMethod::Backward => {
            let mut next = f64::NAN;
            for v in filled.iter_mut().rev() {
                if v.is_nan() {
                    *v = next;
                } else {
                    next = *v;
                }
            }
        }
        FillMethod::Linear => {
            linear_interpolate(&mut filled);
        }
    }

    // Rebuild series with filled values
    TimeSeries {
        name: series.name.clone(),
        points: series
            .points
            .iter()
            .zip(filled.iter())
            .map(|(p, v)| crate::DataPoint {
                timestamp: p.timestamp.clone(),
                value: *v,
            })
            .collect(),
    }
}

/// Linear interpolation of NaN values in-place.
fn linear_interpolate(values: &mut [f64]) {
    let n = values.len();
    if n < 2 {
        return;
    }
    let mut i = 0;
    while i < n {
        if values[i].is_nan() {
            // Find the extent of the NaN run
            let start = i;
            while i < n && values[i].is_nan() {
                i += 1;
            }
            let end = i; // first non-NaN after the run (or n)
                         // Find previous non-NaN
            let prev = if start > 0 {
                Some(values[start - 1])
            } else {
                None
            };
            let next = if end < n { Some(values[end]) } else { None };
            let gap_len = end - start;
            match (prev, next) {
                (Some(p), Some(nv)) => {
                    let step = (nv - p) / (gap_len + 1) as f64;
                    for j in 0..gap_len {
                        values[start + j] = p + step * (j + 1) as f64;
                    }
                }
                (Some(p), None) => {
                    for j in 0..gap_len {
                        values[start + j] = p;
                    }
                }
                (None, Some(nv)) => {
                    for j in 0..gap_len {
                        values[start + j] = nv;
                    }
                }
                (None, None) => {
                    for j in 0..gap_len {
                        values[start + j] = 0.0;
                    }
                }
            }
        } else {
            i += 1;
        }
    }
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
    fn test_fill_forward() {
        let s = make_series(&[1.0, f64::NAN, f64::NAN, 4.0, 5.0]);
        let filled = fill_nans(&s, FillMethod::Forward);
        let vals = filled.values();
        assert!((vals[1] - 1.0).abs() < 1e-9);
        assert!((vals[2] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_fill_backward() {
        let s = make_series(&[1.0, f64::NAN, f64::NAN, 4.0, 5.0]);
        let filled = fill_nans(&s, FillMethod::Backward);
        let vals = filled.values();
        assert!((vals[1] - 4.0).abs() < 1e-9);
        assert!((vals[2] - 4.0).abs() < 1e-9);
    }

    #[test]
    fn test_fill_linear() {
        let s = make_series(&[1.0, f64::NAN, f64::NAN, 4.0, 5.0]);
        let filled = fill_nans(&s, FillMethod::Linear);
        let vals = filled.values();
        assert!((vals[1] - 2.0).abs() < 1e-9);
        assert!((vals[2] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_fill_zero() {
        let s = make_series(&[1.0, f64::NAN, 3.0]);
        let filled = fill_nans(&s, FillMethod::Zero);
        assert!((filled.values()[1] - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_fill_mean() {
        let s = make_series(&[2.0, f64::NAN, 4.0]);
        let filled = fill_nans(&s, FillMethod::Mean);
        assert!((filled.values()[1] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_no_nans() {
        let s = make_series(&[1.0, 2.0, 3.0]);
        let filled = fill_nans(&s, FillMethod::Linear);
        assert_eq!(filled.values(), [1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_detect_gaps() {
        let pairs = vec![
            ("0".to_string(), 1.0),
            ("1".to_string(), 2.0),
            ("2".to_string(), 3.0),
            ("10".to_string(), 4.0),
            ("11".to_string(), 5.0),
            ("12".to_string(), 6.0),
        ];
        let s = TimeSeries::from_pairs("test", pairs);
        let gaps = detect_gaps(&s);
        assert!(!gaps.is_empty());
        assert!(gaps[0].gap_size > 0);
    }

    #[test]
    fn test_linear_interpolate_edges() {
        let s = make_series(&[f64::NAN, f64::NAN, 3.0, 4.0, f64::NAN]);
        let filled = fill_nans(&s, FillMethod::Linear);
        let vals = filled.values();
        assert!((vals[0] - 3.0).abs() < 1e-9);
        assert!((vals[1] - 3.0).abs() < 1e-9);
        assert!((vals[4] - 4.0).abs() < 1e-9);
    }
}
