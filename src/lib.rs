//! # tseries — Time Series Analysis CLI
//!
//! A fast time series analysis library and CLI tool for statistics,
//! anomaly detection, forecasting, seasonality detection, and
//! ASCII visualization.
//!
//! ## Core Types
//!
//! - [`TimeSeries`] — A named series of timestamped values
//! - [`DataPoint`] — A single timestamp-value pair
//! - [`Stats`] — Computed statistics for a series

pub mod anomaly;
pub mod forecast;
pub mod gapfill;
pub mod output;
pub mod reader;
pub mod resample;
pub mod seasonality;
pub mod stats;
pub mod trend;
pub mod visualize;

/// A single timestamped data point.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DataPoint {
    /// ISO-8601 timestamp or Unix epoch seconds (stored as string for flexibility).
    pub timestamp: String,
    /// Numeric value.
    pub value: f64,
}

/// A named time series with metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimeSeries {
    /// Human-readable name for the series.
    pub name: String,
    /// Ordered data points (sorted by timestamp).
    pub points: Vec<DataPoint>,
}

impl TimeSeries {
    /// Create a new empty named series.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            points: Vec::new(),
        }
    }

    /// Create a series from raw (timestamp, value) pairs.
    pub fn from_pairs(name: impl Into<String>, pairs: Vec<(String, f64)>) -> Self {
        let points = pairs
            .into_iter()
            .map(|(ts, v)| DataPoint {
                timestamp: ts,
                value: v,
            })
            .collect();
        Self {
            name: name.into(),
            points,
        }
    }

    /// Number of data points.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Whether the series is empty.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Extract just the values as a vector.
    pub fn values(&self) -> Vec<f64> {
        self.points.iter().map(|p| p.value).collect()
    }

    /// Extract just the timestamps as a vector of string references.
    pub fn timestamps(&self) -> Vec<&str> {
        self.points.iter().map(|p| p.timestamp.as_str()).collect()
    }

    /// Append a data point (caller maintains sort order).
    pub fn push(&mut self, ts: impl Into<String>, value: f64) {
        self.points.push(DataPoint {
            timestamp: ts.into(),
            value,
        });
    }

    /// Sort points by parsed timestamp (best-effort: numeric or ISO-8601).
    pub fn sort_by_timestamp(&mut self) {
        self.points.sort_by_key(|a| parse_ts_key(&a.timestamp));
    }

    /// Minimum value.
    pub fn min(&self) -> Option<f64> {
        self.values().into_iter().reduce(f64::min)
    }

    /// Maximum value.
    pub fn max(&self) -> Option<f64> {
        self.values().into_iter().reduce(f64::max)
    }
}

/// Best-effort sort key for a timestamp string.
fn parse_ts_key(ts: &str) -> SortKey {
    if let Ok(n) = ts.parse::<f64>() {
        return SortKey::Numeric(n);
    }
    SortKey::Raw(ts.to_string())
}

/// Sort key wrapper for mixed timestamp formats.
#[derive(Debug, Clone, PartialEq)]
enum SortKey {
    Numeric(f64),
    Raw(String),
}

// Manual Eq/Ord because f64 doesn't implement Ord.
impl Eq for SortKey {}
impl PartialOrd for SortKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for SortKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (SortKey::Numeric(a), SortKey::Numeric(b)) => {
                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
            }
            (SortKey::Numeric(_), SortKey::Raw(_)) => std::cmp::Ordering::Less,
            (SortKey::Raw(_), SortKey::Numeric(_)) => std::cmp::Ordering::Greater,
            (SortKey::Raw(a), SortKey::Raw(b)) => a.cmp(b),
        }
    }
}
