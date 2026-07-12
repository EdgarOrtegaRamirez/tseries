# tseries

**Fast time series analysis CLI** — statistics, anomaly detection, forecasting, seasonality, trend analysis, gap filling, resampling, and ASCII visualization.

Built in Rust for speed and reliability. Handles CSV, JSON, and JSONL input formats.

## Features

- **Descriptive Statistics** — mean, median, std dev, variance, min/max, range, IQR, CV, skewness, kurtosis, percentiles
- **Anomaly Detection** — Z-score, Modified Z-score (MAD-based), and IQR methods
- **Trend Analysis** — linear regression, growth rate, correlation, decomposition
- **Forecasting** — Simple Exponential Smoothing (SES), Simple Moving Average (SMA), Holt's Linear Trend, Naive, Seasonal Naive
- **Seasonality Detection** — autocorrelation, partial autocorrelation (PACF)
- **Gap Filling** — linear interpolation, forward/backward fill, mean fill, zero fill
- **Resampling** — downsample (mean, sum, min, max, median, first, last, count) and upsample
- **ASCII Visualization** — sparklines, line charts, histograms, box plots
- **Output Formats** — text (default), JSON, CSV

## Installation

### From source

```bash
# Requires Rust 1.70+
git clone https://github.com/EdgarOrtegaRamirez/tseries.git
cd tseries
cargo build --release
```

The binary will be at `target/release/tseries`. Optionally copy it to your PATH:

```bash
cp target/release/tseries ~/.local/bin/
```

## Usage

```
tseries [OPTIONS] <COMMAND>

Commands:
  stats        Show descriptive statistics
  anomaly      Detect anomalies
  trend        Analyze trend (linear regression)
  forecast     Forecast future values
  seasonality  Detect seasonality
  fill         Fill gaps / NaN values
  resample     Resample (downsample or upsample)
  plot         Visualize as ASCII
  show         Show the raw series data
  help         Print this message or the help of the given subcommand(s)

Options:
  -i, --input <INPUT>    Input file path (reads from stdin if omitted)
  -n, --name <NAME>      Name for the series [default: series]
  -o, --output <OUTPUT>  Output format [default: text] [possible values: text, json, csv]
  -h, --help             Print help
  -V, --version          Print version
```

### Examples

**Basic statistics:**
```bash
tseries -i data.csv stats
```

**Detect anomalies using IQR:**
```bash
tseries -i data.csv anomaly --method iqr --iqr 1.5
```

**Forecast the next 10 values using SES:**
```bash
tseries -i data.csv forecast --method ses --horizon 10
```

**Detect seasonality:**
```bash
tseries -i data.csv seasonality
```

**Plot a sparkline:**
```bash
tseries -i data.csv plot --plot-type sparkline
```

**Line chart with custom dimensions:**
```bash
tseries -i data.csv plot --plot-type line --height 15 --width 80
```

**JSON output:**
```bash
tseries -i data.csv --output json stats
```

**CSV output:**
```bash
tseries -i data.csv --output csv stats
```

**Fill missing values with linear interpolation:**
```bash
tseries -i data.csv fill --method linear
```

**Resample (downsample by factor 3 using max aggregation):**
```bash
tseries -i data.csv resample --factor 3 --direction down --aggregation max
```

### Input Format

CSV files should have a `timestamp` (or `date`/`time`) column and a `value` column:

```csv
timestamp,value
2024-01-01T00:00:00Z,10.5
2024-01-02T00:00:00Z,12.3
2024-01-03T00:00:00Z,11.8
```

JSON array format:
```json
[{"timestamp": "2024-01-01", "value": 10.5}, {"timestamp": "2024-01-02", "value": 12.3}]
```

JSONL format (one JSON object per line):
```jsonl
{"timestamp": "2024-01-01", "value": 10.5}
{"timestamp": "2024-01-02", "value": 12.3}
```

## Architecture

```
┌─────────────┐     ┌───────────┐     ┌──────────┐
│  CLI (clap) │────▶│  Commands │────▶│  Output  │
│  main.rs    │     │  dispatch │     │  formatter│
└─────────────┘     └─────┬─────┘     └──────────┘
                          │
          ┌───────────────┼───────────────┐
          ▼               ▼               ▼
    ┌──────────┐   ┌──────────┐   ┌──────────────┐
    │  Reader  │   │  Stats   │   │  Anomaly     │
    │  reader/ │   │  stats/  │   │  anomaly/    │
    └──────────┘   └──────────┘   └──────────────┘
    ┌──────────┐   ┌──────────┐   ┌──────────────┐
    │ Forecast │   │  Trend   │   │  Seasonality │
    │ forecast/│   │  trend/  │   │  seasonality/│
    └──────────┘   └──────────┘   └──────────────┘
    ┌──────────┐   ┌──────────┐   ┌──────────────┐
    │ GapFill  │   │ Resample │   │  Visualize   │
    │ gapfill/ │   │ resample/│   │  visualize/  │
    └──────────┘   └──────────┘   └──────────────┘
```

### Module Overview

| Module | Description |
|--------|-------------|
| `reader` | Parse CSV, JSON, and JSONL into TimeSeries |
| `stats` | Descriptive statistics, percentiles, moving averages |
| `anomaly` | Z-score, modified z-score, IQR outlier detection |
| `trend` | Linear regression, growth rate, correlation |
| `forecast` | SES, SMA, Holt, Naive, Seasonal Naive |
| `seasonality` | Autocorrelation, partial autocorrelation |
| `gapfill` | NaN/missing value interpolation and filling |
| `resample` | Downsampling and upsampling with various aggregations |
| `visualize` | ASCII sparklines, line charts, histograms, box plots |
| `output` | Format results as text, JSON, or CSV |

## Development

```bash
# Run tests (unit + integration)
cargo test

# Format code
cargo fmt

# Lint
cargo clippy -- -D warnings

# Build release
cargo build --release
```

Requires Rust 1.70+.

## License

MIT