//! Input parsing for CSV, JSON, and JSONL formats.

use crate::{DataPoint, TimeSeries};
use std::io::Read;

/// Supported input formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Csv,
    Json,
    Jsonl,
}

/// Parse a time series from a reader, auto-detecting the format.
pub fn read_series<R: Read>(reader: R, name: &str) -> Result<TimeSeries, ReadError> {
    let mut buf = String::new();
    let mut reader = reader;
    reader
        .read_to_string(&mut buf)
        .map_err(|e| ReadError::Io(e.to_string()))?;
    let trimmed = buf.trim();
    if trimmed.is_empty() {
        return Err(ReadError::Empty);
    }
    let format = detect_format(trimmed);
    match format {
        Format::Csv => parse_csv(trimmed, name),
        Format::Json => parse_json(trimmed, name),
        Format::Jsonl => parse_jsonl(trimmed, name),
    }
}

/// Parse a time series from a file path.
pub fn read_file(path: &str, name: &str) -> Result<TimeSeries, ReadError> {
    let content = std::fs::read_to_string(path).map_err(|e| ReadError::Io(e.to_string()))?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err(ReadError::Empty);
    }
    let format = detect_format(trimmed);
    match format {
        Format::Csv => parse_csv(trimmed, name),
        Format::Json => parse_json(trimmed, name),
        Format::Jsonl => parse_jsonl(trimmed, name),
    }
}

/// Auto-detect format from content.
fn detect_format(content: &str) -> Format {
    let trimmed = content.trim_start();
    if trimmed.starts_with('[') {
        return Format::Json;
    }
    if trimmed.starts_with('{') {
        // If multiple lines each starting with '{', treat as JSONL
        let lines: Vec<&str> = trimmed.lines().filter(|l| !l.trim().is_empty()).collect();
        if lines.len() > 1 && lines.iter().all(|l| l.trim_start().starts_with('{')) {
            return Format::Jsonl;
        }
        return Format::Json;
    }
    // Default to CSV for anything else
    Format::Csv
}

/// Parse CSV with timestamp,value columns.
/// Expects at least 2 columns. First column = timestamp, second = value.
/// Skips header row if the value column is non-numeric.
fn parse_csv(content: &str, name: &str) -> Result<TimeSeries, ReadError> {
    let mut points = Vec::new();
    let mut has_header = false;

    for (i, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Simple CSV split (handles basic quoting)
        let cols = csv_split(line);
        if cols.len() < 2 {
            return Err(ReadError::Parse(format!(
                "line {}: expected at least 2 columns, got {}",
                i + 1,
                cols.len()
            )));
        }
        let ts = cols[0].trim().to_string();
        let val_str = cols[1].trim();
        let val: f64 = match val_str.parse() {
            Ok(v) => v,
            Err(_) => {
                if i == 0 {
                    has_header = true;
                    continue;
                }
                return Err(ReadError::Parse(format!(
                    "line {}: invalid value '{}'",
                    i + 1,
                    val_str
                )));
            }
        };
        points.push(DataPoint {
            timestamp: ts,
            value: val,
        });
    }

    if points.is_empty() {
        return Err(ReadError::Empty);
    }

    let _ = has_header; // header auto-skipped
    let mut series = TimeSeries {
        name: name.to_string(),
        points,
    };
    series.sort_by_timestamp();
    Ok(series)
}

/// Parse JSON array of {"timestamp": "...", "value": ...} objects.
fn parse_json(content: &str, name: &str) -> Result<TimeSeries, ReadError> {
    let v: serde_json::Value =
        serde_json::from_str(content).map_err(|e| ReadError::Parse(format!("JSON: {e}")))?;

    let points = match &v {
        serde_json::Value::Array(arr) => {
            let mut pts = Vec::with_capacity(arr.len());
            for item in arr {
                pts.push(json_to_point(item)?);
            }
            pts
        }
        serde_json::Value::Object(map) => {
            // Single object with "points" array or direct timestamp/value
            if let Some(arr) = map.get("points").and_then(|p| p.as_array()) {
                let mut pts = Vec::with_capacity(arr.len());
                for item in arr {
                    pts.push(json_to_point(item)?);
                }
                pts
            } else {
                vec![json_to_point(&v)?]
            }
        }
        _ => return Err(ReadError::Parse("expected JSON array or object".into())),
    };

    if points.is_empty() {
        return Err(ReadError::Empty);
    }
    let mut series = TimeSeries {
        name: name.to_string(),
        points,
    };
    series.sort_by_timestamp();
    Ok(series)
}

/// Parse JSONL (one JSON object per line).
fn parse_jsonl(content: &str, name: &str) -> Result<TimeSeries, ReadError> {
    let mut points = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(line)
            .map_err(|e| ReadError::Parse(format!("line {}: JSON: {e}", i + 1)))?;
        points.push(json_to_point(&v)?);
    }
    if points.is_empty() {
        return Err(ReadError::Empty);
    }
    let mut series = TimeSeries {
        name: name.to_string(),
        points,
    };
    series.sort_by_timestamp();
    Ok(series)
}

fn json_to_point(v: &serde_json::Value) -> Result<DataPoint, ReadError> {
    let map = v
        .as_object()
        .ok_or_else(|| ReadError::Parse("expected JSON object".into()))?;

    let ts = map
        .get("timestamp")
        .or_else(|| map.get("time"))
        .or_else(|| map.get("date"))
        .or_else(|| map.get("ts"))
        .ok_or_else(|| ReadError::Parse("missing 'timestamp' field".into()))?;

    let timestamp = match ts {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => {
            return Err(ReadError::Parse(
                "timestamp must be string or number".into(),
            ))
        }
    };

    let val = map
        .get("value")
        .or_else(|| map.get("val"))
        .or_else(|| map.get("v"))
        .ok_or_else(|| ReadError::Parse("missing 'value' field".into()))?;

    let value = val
        .as_f64()
        .ok_or_else(|| ReadError::Parse("value must be a number".into()))?;

    Ok(DataPoint { timestamp, value })
}

/// Simple CSV field splitter handling basic double-quote escaping.
fn csv_split(line: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            if in_quotes && chars.peek() == Some(&'"') {
                current.push('"');
                chars.next();
            } else {
                in_quotes = !in_quotes;
            }
        } else if c == ',' && !in_quotes {
            result.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    result.push(current);
    result
}

/// Errors that can occur during reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    Empty,
    Io(String),
    Parse(String),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::Empty => write!(f, "input is empty"),
            ReadError::Io(msg) => write!(f, "IO error: {msg}"),
            ReadError::Parse(msg) => write!(f, "parse error: {msg}"),
        }
    }
}

impl std::error::Error for ReadError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csv_basic() {
        let csv = "timestamp,value\n2024-01-01,10.5\n2024-01-02,20.3\n2024-01-03,15.0";
        let s = parse_csv(csv, "test").unwrap();
        assert_eq!(s.len(), 3);
        assert!((s.points[0].value - 10.5).abs() < 1e-9);
    }

    #[test]
    fn test_parse_csv_no_header() {
        let csv = "2024-01-01,10.5\n2024-01-02,20.3";
        let s = parse_csv(csv, "test").unwrap();
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn test_parse_csv_quoted() {
        let csv = "\"2024-01-01\",10.5\n\"2024-01-02\",20.3";
        let s = parse_csv(csv, "test").unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!(s.points[0].timestamp, "2024-01-01");
    }

    #[test]
    fn test_parse_json_array() {
        let json =
            r#"[{"timestamp":"2024-01-01","value":10.5},{"timestamp":"2024-01-02","value":20.3}]"#;
        let s = parse_json(json, "test").unwrap();
        assert_eq!(s.len(), 2);
        assert!((s.points[0].value - 10.5).abs() < 1e-9);
    }

    #[test]
    fn test_parse_json_numeric_ts() {
        let json =
            r#"[{"timestamp":1704067200,"value":10.5},{"timestamp":1704153600,"value":20.3}]"#;
        let s = parse_json(json, "test").unwrap();
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn test_parse_jsonl() {
        let jsonl = "{\"timestamp\":\"2024-01-01\",\"value\":10.5}\n{\"timestamp\":\"2024-01-02\",\"value\":20.3}";
        let s = parse_jsonl(jsonl, "test").unwrap();
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn test_parse_empty() {
        assert!(parse_csv("", "test").is_err());
    }

    #[test]
    fn test_detect_format() {
        assert_eq!(detect_format("[...]"), Format::Json);
        assert_eq!(detect_format("a,b\n1,2"), Format::Csv);
        assert_eq!(
            detect_format("{\"ts\":\"x\"}\n{\"ts\":\"y\"}"),
            Format::Jsonl
        );
    }

    #[test]
    fn test_csv_split() {
        assert_eq!(csv_split("a,b,c"), ["a", "b", "c"]);
        assert_eq!(csv_split("\"a,b\",c"), ["a,b", "c"]);
        assert_eq!(csv_split("\"a\"\"b\",c"), ["a\"b", "c"]);
    }

    #[test]
    fn test_alternate_field_names() {
        let json = r#"[{"time":"2024-01-01","val":10.5}]"#;
        let s = parse_json(json, "test").unwrap();
        assert_eq!(s.len(), 1);
        assert!((s.points[0].value - 10.5).abs() < 1e-9);
    }
}
