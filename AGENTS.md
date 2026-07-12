# AGENTS.md — for AI Agents

## Overview

`tseries` is a Rust CLI tool for time series analysis. It reads CSV, JSON, and JSONL data and provides statistics, anomaly detection, forecasting, trend analysis, seasonality detection, gap filling, resampling, and ASCII visualization.

## Build & Test

```bash
cargo build --release    # release build
cargo test               # run all tests (unit + integration)
cargo fmt                # format code
cargo clippy -- -D warnings  # lint
```

## Project Structure

```
src/
  main.rs          — CLI entry point (clap definitions, dispatch)
  lib.rs           — library root, TimeSeries type, module exports
  reader.rs        — CSV/JSON/JSONL file parsing
  stats.rs         — descriptive statistics, percentiles
  anomaly.rs       — anomaly detection (zscore, modified zscore, IQR)
  trend.rs         — linear regression, growth rate, correlation
  forecast.rs      — forecasting (SES, SMA, Holt, Naive, Seasonal Naive)
  seasonality.rs   — autocorrelation, partial autocorrelation
  gapfill.rs       — missing value filling (linear, forward, backward, mean, zero)
  resample.rs      — down/up sampling with aggregations
  visualize.rs     — ASCII sparkline, line chart, histogram, box plot
  output.rs        — output formatting (text, JSON, CSV)
tests/
  integration.rs   — CLI integration tests (cargo test --test integration)
```

## Key Types

- `TimeSeries` — holds a `name: String` and `points: Vec<DataPoint>` (each with `timestamp: String`, `value: f64`)
- `Anomaly` — detected outlier with method, score, threshold, timestamp, value
- `Forecast` — forecasted values with error metrics (MAE, RMSE)

## Dependencies

- `clap` 4.5 — CLI argument parsing (derive)
- `serde` / `serde_json` 1.0 — JSON serialization
- `chrono` 0.4 — timestamp parsing (serde feature)

Dev dependencies:
- `tempfile` — temporary test files
- `assert_cmd`, `predicates` — CLI testing (optional, tests use `Command` directly)

## CI

GitHub Actions workflow at `.github/workflows/ci.yml` — runs `cargo test` on push/PR to main.

## No External AI Dependencies

This project does not use any AI APIs. All analysis is purely algorithmic.