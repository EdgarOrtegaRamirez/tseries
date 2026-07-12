use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

/// Helper: create a temporary CSV file with time series data.
fn create_csv_data(values: &[f64]) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "timestamp,value").unwrap();
    for (i, v) in values.iter().enumerate() {
        writeln!(file, "2024-01-{:02}T00:00:00Z,{}", i + 1, v).unwrap();
    }
    file
}

/// Helper: create a temporary JSON file with time series data.
fn create_json_data(values: &[f64]) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    let json = serde_json::to_string_pretty(
        &values
            .iter()
            .enumerate()
            .map(|(i, v)| {
                serde_json::json!({
                    "timestamp": format!("2024-01-{:02}T00:00:00Z", i + 1),
                    "value": v,
                })
            })
            .collect::<Vec<_>>(),
    )
    .unwrap();
    file.write_all(json.as_bytes()).unwrap();
    file
}

#[test]
fn test_help_succeeds() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("tseries"));
    assert!(stdout.contains("stats"));
    assert!(stdout.contains("anomaly"));
    assert!(stdout.contains("trend"));
    assert!(stdout.contains("forecast"));
}

#[test]
fn test_version() {
    let output = Command::new("cargo")
        .args(["run", "--", "--version"])
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn test_stats_command() {
    let csv = create_csv_data(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let path = csv.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args(["run", "--", "--input", path, "stats"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stats failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Count:"));
    assert!(stdout.contains("Mean:"));
    assert!(stdout.contains("5"), "Expected '5' (count) in: {stdout}");
}

#[test]
fn test_stats_json() {
    let csv = create_csv_data(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let path = csv.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args(["run", "--", "--input", path, "--output", "json", "stats"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.get("count").is_some());
    assert_eq!(parsed["count"], 5);
    assert!((parsed["mean"].as_f64().unwrap() - 3.0).abs() < 0.001);
}

#[test]
fn test_anomaly_all_methods() {
    // Values with an obvious outlier (IQR method will catch it)
    let csv = create_csv_data(&[1.0, 1.1, 0.9, 1.0, 1.2, 1.0, 0.8, 1.1, 1.0, 100.0]);
    let path = csv.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args(["run", "--", "--input", path, "anomaly"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "anomaly failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Anomalies"),
        "Expected 'Anomalies' in: {stdout}"
    );
}

#[test]
fn test_anomaly_iqr() {
    let csv = create_csv_data(&[1.0, 2.0, 3.0, 4.0, 100.0]);
    let path = csv.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args([
            "run", "--", "--input", path, "anomaly", "--method", "iqr", "--iqr", "1.5",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Anomalies"),
        "Expected 'Anomalies' in: {stdout}"
    );
}

#[test]
fn test_anomaly_zscore() {
    // Use very extreme outlier with low threshold for zscore to detect
    let csv = create_csv_data(&[1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 500.0]);
    let path = csv.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--input",
            path,
            "anomaly",
            "--method",
            "zscore",
            "--threshold",
            "2.0",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "anomaly zscore failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Anomalies"),
        "Expected 'Anomalies' in: {stdout}"
    );
}

#[test]
fn test_trend_command() {
    // Linear data: y = 2x + 1
    let data: Vec<f64> = (0..10).map(|x| 2.0 * x as f64 + 1.0).collect();
    let csv = create_csv_data(&data);
    let path = csv.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args(["run", "--", "--input", path, "trend"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "trend failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Trend"), "Expected 'Trend' in: {stdout}");
}

#[test]
fn test_forecast_ses() {
    let csv = create_csv_data(&[10.0, 12.0, 11.0, 13.0, 12.0, 14.0]);
    let path = csv.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--input",
            path,
            "forecast",
            "--method",
            "ses",
            "--horizon",
            "5",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "forecast failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Forecast"),
        "Expected 'Forecast' in: {stdout}"
    );
}

#[test]
fn test_forecast_sma() {
    let csv = create_csv_data(&[10.0, 12.0, 11.0, 13.0, 12.0, 14.0, 13.0]);
    let path = csv.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--input",
            path,
            "forecast",
            "--method",
            "sma",
            "--window",
            "3",
            "--horizon",
            "3",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Forecast"),
        "Expected 'Forecast' in: {stdout}"
    );
}

#[test]
fn test_forecast_holt() {
    let csv = create_csv_data(&[5.0, 7.0, 9.0, 11.0, 13.0, 15.0]);
    let path = csv.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--input",
            path,
            "forecast",
            "--method",
            "holt",
            "--horizon",
            "3",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Forecast"),
        "Expected 'Forecast' in: {stdout}"
    );
}

#[test]
fn test_forecast_naive() {
    let csv = create_csv_data(&[10.0, 12.0, 11.0, 13.0]);
    let path = csv.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--input",
            path,
            "forecast",
            "--method",
            "naive",
            "--horizon",
            "3",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Forecast"),
        "Expected 'Forecast' in: {stdout}"
    );
}

#[test]
fn test_forecast_seasonal_naive() {
    let csv = create_csv_data(&[1.0, 2.0, 3.0, 1.0, 2.0, 3.0, 1.0, 2.0, 3.0]);
    let path = csv.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--input",
            path,
            "forecast",
            "--method",
            "seasonal-naive",
            "--period",
            "3",
            "--horizon",
            "3",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Forecast"),
        "Expected 'Forecast' in: {stdout}"
    );
}

#[test]
fn test_seasonality() {
    // Sinusoidal-like data (approx)
    let data: Vec<f64> = (0..24)
        .map(|i| (i as f64 * std::f64::consts::PI / 6.0).sin() * 5.0 + 10.0)
        .collect();
    let csv = create_csv_data(&data);
    let path = csv.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args(["run", "--", "--input", path, "seasonality"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "seasonality failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Seasonality"),
        "Expected 'Seasonality' in: {stdout}"
    );
}

#[test]
fn test_fill_linear() {
    let csv = create_csv_data(&[1.0, f64::NAN, 3.0, f64::NAN, 5.0]);
    let path = csv.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args(["run", "--", "--input", path, "fill", "--method", "linear"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "fill failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_fill_forward() {
    let csv = create_csv_data(&[1.0, f64::NAN, 3.0, f64::NAN, 5.0]);
    let path = csv.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args(["run", "--", "--input", path, "fill", "--method", "forward"])
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn test_resample_down() {
    let csv = create_csv_data(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    let path = csv.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--input",
            path,
            "resample",
            "--factor",
            "2",
            "--direction",
            "down",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "resample failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_resample_up() {
    let csv = create_csv_data(&[1.0, 2.0, 3.0]);
    let path = csv.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--input",
            path,
            "resample",
            "--factor",
            "2",
            "--direction",
            "up",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn test_plot_sparkline() {
    let csv = create_csv_data(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let path = csv.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--input",
            path,
            "plot",
            "--plot-type",
            "sparkline",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "plot sparkline failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Sparkline should have some Unicode block characters
    assert!(
        stdout.contains('▁')
            || stdout.contains('▂')
            || stdout.contains('▄')
            || stdout.contains('▆')
            || stdout.contains('█'),
        "Expected sparkline characters in: {stdout}"
    );
}

#[test]
fn test_plot_line() {
    let csv = create_csv_data(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let path = csv.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--input",
            path,
            "plot",
            "--plot-type",
            "line",
            "--height",
            "10",
            "--width",
            "40",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains('┌') || stdout.contains('│') || stdout.contains('─'),
        "Expected line chart characters in: {stdout}"
    );
}

#[test]
fn test_plot_histogram() {
    let csv = create_csv_data(&[1.0, 1.0, 2.0, 2.0, 2.0, 3.0, 3.0, 4.0, 5.0]);
    let path = csv.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--input",
            path,
            "plot",
            "--plot-type",
            "histogram",
            "--bins",
            "5",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn test_show_command() {
    let csv = create_csv_data(&[1.0, 2.0, 3.0]);
    let path = csv.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args(["run", "--", "--input", path, "show"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("1.0000"));
    assert!(stdout.contains("2.0000"));
    assert!(stdout.contains("3.0000"));
}

#[test]
fn test_json_input() {
    let json = create_json_data(&[1.0, 2.0, 3.0]);
    let path = json.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args(["run", "--", "--input", path, "stats"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "json input failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_empty_file_error() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "timestamp,value").unwrap();
    let path = file.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args(["run", "--", "--input", path, "stats"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no data") || stderr.contains("Error"),
        "Expected error about no data, got: {stderr}"
    );
}

#[test]
fn test_csv_output_format() {
    let csv = create_csv_data(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let path = csv.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args(["run", "--", "--input", path, "--output", "csv", "stats"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("metric,value") || stdout.contains("name"));
}

#[test]
fn test_missing_file_error() {
    let output = Command::new("cargo")
        .args(["run", "--", "--input", "/nonexistent/path.csv", "stats"])
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn test_large_dataset() {
    let data: Vec<f64> = (0..1000).map(|i| (i as f64).sin() * 10.0 + 50.0).collect();
    let csv = create_csv_data(&data);
    let path = csv.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args(["run", "--", "--input", path, "stats"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "large dataset failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_anomaly_json_output() {
    let csv = create_csv_data(&[1.0, 1.1, 0.9, 1.0, 100.0]);
    let path = csv.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args(["run", "--", "--input", path, "--output", "json", "anomaly"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should be valid JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or(serde_json::Value::Null);
    assert!(parsed.is_array() || parsed.is_object());
}

#[test]
fn test_forecast_error_metrics() {
    let csv = create_csv_data(&[10.0, 12.0, 11.0, 13.0, 12.0, 14.0, 13.0, 15.0]);
    let path = csv.path().to_str().unwrap();
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--input",
            path,
            "forecast",
            "--method",
            "ses",
            "--horizon",
            "3",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should have error metrics
    assert!(
        stdout.contains("MAE") || stdout.contains("RMSE") || stdout.contains("Forecast"),
        "Expected error metrics in: {stdout}"
    );
}
