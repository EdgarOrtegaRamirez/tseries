//! Resampling: downsample (aggregate) and upsample (interpolate) time series.

use crate::stats;
use crate::TimeSeries;

/// Aggregation function for downsampling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Aggregation {
    Mean,
    Sum,
    Min,
    Max,
    Median,
    First,
    Last,
    Count,
}

impl Aggregation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Aggregation::Mean => "mean",
            Aggregation::Sum => "sum",
            Aggregation::Min => "min",
            Aggregation::Max => "max",
            Aggregation::Median => "median",
            Aggregation::First => "first",
            Aggregation::Last => "last",
            Aggregation::Count => "count",
        }
    }

    /// Apply aggregation to a slice of values.
    pub fn apply(&self, values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        match self {
            Aggregation::Mean => values.iter().sum::<f64>() / values.len() as f64,
            Aggregation::Sum => values.iter().sum(),
            Aggregation::Min => values.iter().cloned().fold(f64::INFINITY, f64::min),
            Aggregation::Max => values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            Aggregation::Median => {
                let mut s = values.to_vec();
                s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                stats::percentile_sorted(&s, 50.0)
            }
            Aggregation::First => values[0],
            Aggregation::Last => values[values.len() - 1],
            Aggregation::Count => values.len() as f64,
        }
    }
}

/// Downsample a series by grouping consecutive points into buckets of `factor` size
/// and applying an aggregation function.
pub fn downsample(series: &TimeSeries, factor: usize, agg: Aggregation) -> TimeSeries {
    if factor == 0 || factor >= series.points.len() {
        return series.clone();
    }
    let mut new_points = Vec::new();
    for chunk in series.points.chunks(factor) {
        let values: Vec<f64> = chunk.iter().map(|p| p.value).collect();
        let agg_value = agg.apply(&values);
        // Use the first timestamp in the bucket
        new_points.push(crate::DataPoint {
            timestamp: chunk[0].timestamp.clone(),
            value: agg_value,
        });
    }
    TimeSeries {
        name: series.name.clone(),
        points: new_points,
    }
}

/// Upsample a series by inserting interpolated points between existing ones.
/// Each gap between consecutive points is filled with `factor - 1` interpolated values.
pub fn upsample(series: &TimeSeries, factor: usize) -> TimeSeries {
    if factor <= 1 || series.points.len() < 2 {
        return series.clone();
    }
    let mut new_points = Vec::new();
    for window in series.points.windows(2) {
        let (p0, p1) = (&window[0], &window[1]);
        new_points.push(p0.clone());
        for j in 1..factor {
            let frac = j as f64 / factor as f64;
            let interp_value = p0.value * (1.0 - frac) + p1.value * frac;
            // Interpolate timestamp if numeric
            let interp_ts = if let (Ok(t0), Ok(t1)) =
                (p0.timestamp.parse::<f64>(), p1.timestamp.parse::<f64>())
            {
                (t0 * (1.0 - frac) + t1 * frac).to_string()
            } else {
                format!("{}_{}", p0.timestamp, j)
            };
            new_points.push(crate::DataPoint {
                timestamp: interp_ts,
                value: interp_value,
            });
        }
    }
    // Don't forget the last point
    if let Some(last) = series.points.last() {
        new_points.push(last.clone());
    }
    TimeSeries {
        name: series.name.clone(),
        points: new_points,
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
    fn test_downsample_mean() {
        let s = make_series(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let ds = downsample(&s, 2, Aggregation::Mean);
        assert_eq!(ds.len(), 3);
        assert!((ds.values()[0] - 1.5).abs() < 1e-9);
        assert!((ds.values()[1] - 3.5).abs() < 1e-9);
        assert!((ds.values()[2] - 5.5).abs() < 1e-9);
    }

    #[test]
    fn test_downsample_sum() {
        let s = make_series(&[1.0, 2.0, 3.0, 4.0]);
        let ds = downsample(&s, 2, Aggregation::Sum);
        assert_eq!(ds.values(), [3.0, 7.0]);
    }

    #[test]
    fn test_downsample_max() {
        let s = make_series(&[1.0, 5.0, 3.0, 2.0]);
        let ds = downsample(&s, 2, Aggregation::Max);
        assert_eq!(ds.values(), [5.0, 3.0]);
    }

    #[test]
    fn test_downsample_uneven() {
        let s = make_series(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let ds = downsample(&s, 2, Aggregation::Mean);
        assert_eq!(ds.len(), 3); // 2+2+1
        assert!((ds.values()[2] - 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_upsample() {
        let s = make_series(&[0.0, 10.0]);
        let us = upsample(&s, 3);
        assert_eq!(us.len(), 4); // original + 2 interpolated + last
        assert!((us.values()[1] - 3.333).abs() < 0.01);
        assert!((us.values()[2] - 6.666).abs() < 0.01);
    }

    #[test]
    fn test_aggregation_count() {
        assert_eq!(Aggregation::Count.apply(&[1.0, 2.0, 3.0]), 3.0);
    }

    #[test]
    fn test_aggregation_median() {
        assert!((Aggregation::Median.apply(&[1.0, 3.0, 2.0]) - 2.0).abs() < 1e-9);
    }
}
