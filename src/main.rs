//! # tseries CLI
//!
//! Time series analysis command-line tool.

use clap::{Parser, Subcommand};
use std::io::Write;
use std::process::ExitCode;
use tseries::anomaly;
use tseries::forecast;
use tseries::gapfill;
use tseries::output;
use tseries::output::OutputFormat;
use tseries::reader;
use tseries::resample;
use tseries::seasonality;
use tseries::stats;
use tseries::trend;
use tseries::visualize;

#[derive(Parser)]
#[command(
    name = "tseries",
    version,
    about = "Fast time series analysis CLI — statistics, anomaly detection, forecasting, and visualization",
    long_about = "A command-line tool for analyzing time series data.\nSupports CSV, JSON, and JSONL input.\nProvides statistics, anomaly detection, forecasting, seasonality detection, trend analysis, and ASCII visualization."
)]
struct Cli {
    /// Input file path. Reads from stdin if omitted.
    #[arg(short, long)]
    input: Option<String>,

    /// Name for the series (used in output labels).
    #[arg(short, long, default_value = "series")]
    name: String,

    /// Output format.
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Text)]
    output: OutputFormat,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show descriptive statistics.
    Stats,
    /// Detect anomalies.
    Anomaly {
        /// Z-score threshold (default 3.0).
        #[arg(long, default_value_t = 3.0)]
        threshold: f64,
        /// IQR multiplier (default 1.5).
        #[arg(long, default_value_t = 1.5)]
        iqr: f64,
        /// Only use a specific method.
        #[arg(long, value_enum)]
        method: Option<AnomalyMethodArg>,
    },
    /// Analyze trend (linear regression).
    Trend,
    /// Forecast future values.
    Forecast {
        /// Number of periods to forecast.
        #[arg(short = 'H', long, default_value_t = 10)]
        horizon: usize,
        /// Forecasting method.
        #[arg(short, long, value_enum, default_value_t = ForecastMethodArg::Ses)]
        method: ForecastMethodArg,
        /// Smoothing alpha (for SES/Holt).
        #[arg(long, default_value_t = 0.3)]
        alpha: f64,
        /// Smoothing beta (for Holt).
        #[arg(long, default_value_t = 0.1)]
        beta: f64,
        /// Window size (for SMA).
        #[arg(long, default_value_t = 5)]
        window: usize,
        /// Seasonal period (for seasonal naive).
        #[arg(long)]
        period: Option<usize>,
    },
    /// Detect seasonality.
    Seasonality,
    /// Fill gaps / NaN values.
    Fill {
        /// Fill method.
        #[arg(short, long, value_enum, default_value_t = FillMethodArg::Linear)]
        method: FillMethodArg,
    },
    /// Resample (downsample or upsample).
    Resample {
        /// Factor for resampling.
        #[arg(short, long, default_value_t = 2)]
        factor: usize,
        /// Direction: down or up.
        #[arg(short, long, value_enum, default_value_t = ResampleDirArg::Down)]
        direction: ResampleDirArg,
        /// Aggregation (for downsampling).
        #[arg(short, long, value_enum, default_value_t = AggregationArg::Mean)]
        aggregation: AggregationArg,
    },
    /// Visualize as ASCII.
    Plot {
        /// Plot type.
        #[arg(short, long, value_enum, default_value_t = PlotTypeArg::Sparkline)]
        plot_type: PlotTypeArg,
        /// Chart height (for line chart).
        #[arg(long, default_value_t = 10)]
        height: usize,
        /// Chart width (for line chart).
        #[arg(long, default_value_t = 60)]
        width: usize,
        /// Number of bins (for histogram).
        #[arg(long, default_value_t = 20)]
        bins: usize,
    },
    /// Show the raw series data.
    Show,
}

#[derive(Clone, clap::ValueEnum)]
enum AnomalyMethodArg {
    Zscore,
    ModifiedZscore,
    Iqr,
    All,
}

#[derive(Clone, clap::ValueEnum)]
enum ForecastMethodArg {
    Naive,
    Ses,
    Sma,
    Holt,
    SeasonalNaive,
}

#[derive(Clone, clap::ValueEnum)]
enum FillMethodArg {
    Linear,
    Forward,
    Backward,
    Mean,
    Zero,
}

#[derive(Clone, clap::ValueEnum)]
enum ResampleDirArg {
    Down,
    Up,
}

#[derive(Clone, clap::ValueEnum)]
enum AggregationArg {
    Mean,
    Sum,
    Min,
    Max,
    Median,
    First,
    Last,
    Count,
}

#[derive(Clone, clap::ValueEnum)]
enum PlotTypeArg {
    Sparkline,
    Line,
    Histogram,
    Box,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let series = match load_series(&cli.input, &cli.name) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {e}");
            return ExitCode::FAILURE;
        }
    };

    if series.is_empty() {
        eprintln!("Error: no data points found");
        return ExitCode::FAILURE;
    }

    let output = match &cli.command {
        Command::Stats => {
            let stats = stats::compute_stats(&series);
            match stats {
                Some(s) => output::format_stats(&s, &cli.name, cli.output),
                None => "Error: cannot compute statistics\n".into(),
            }
        }
        Command::Anomaly {
            threshold,
            iqr,
            method,
        } => {
            let anomalies = match method {
                Some(AnomalyMethodArg::Zscore) => anomaly::detect_zscore(&series, *threshold),
                Some(AnomalyMethodArg::ModifiedZscore) => {
                    anomaly::detect_modified_zscore(&series, *threshold)
                }
                Some(AnomalyMethodArg::Iqr) => anomaly::detect_iqr(&series, *iqr),
                _ => anomaly::detect_all(&series, *threshold, *iqr),
            };
            output::format_anomalies(&anomalies, cli.output)
        }
        Command::Trend => {
            let trend = trend::analyze_trend(&series);
            match trend {
                Some(t) => output::format_trend(&t, cli.output),
                None => "Error: not enough data for trend analysis\n".into(),
            }
        }
        Command::Forecast {
            horizon,
            method,
            alpha,
            beta,
            window,
            period,
        } => {
            let f = match method {
                ForecastMethodArg::Naive => forecast::forecast_naive(&series, *horizon),
                ForecastMethodArg::Ses => forecast::forecast_ses(&series, *alpha, *horizon),
                ForecastMethodArg::Sma => forecast::forecast_sma(&series, *window, *horizon),
                ForecastMethodArg::Holt => {
                    forecast::forecast_holt(&series, *alpha, *beta, *horizon)
                }
                ForecastMethodArg::SeasonalNaive => {
                    let p = period.unwrap_or(1);
                    forecast::forecast_seasonal_naive(&series, p, *horizon)
                }
            };
            output::format_forecast(&f, cli.output)
        }
        Command::Seasonality => {
            let result = seasonality::detect_seasonality(&series);
            output::format_seasonality(&result, cli.output)
        }
        Command::Fill { method } => {
            let m = match method {
                FillMethodArg::Linear => gapfill::FillMethod::Linear,
                FillMethodArg::Forward => gapfill::FillMethod::Forward,
                FillMethodArg::Backward => gapfill::FillMethod::Backward,
                FillMethodArg::Mean => gapfill::FillMethod::Mean,
                FillMethodArg::Zero => gapfill::FillMethod::Zero,
            };
            let filled = gapfill::fill_nans(&series, m);
            output::format_series(&filled, cli.output)
        }
        Command::Resample {
            factor,
            direction,
            aggregation,
        } => match direction {
            ResampleDirArg::Down => {
                let agg = match aggregation {
                    AggregationArg::Mean => resample::Aggregation::Mean,
                    AggregationArg::Sum => resample::Aggregation::Sum,
                    AggregationArg::Min => resample::Aggregation::Min,
                    AggregationArg::Max => resample::Aggregation::Max,
                    AggregationArg::Median => resample::Aggregation::Median,
                    AggregationArg::First => resample::Aggregation::First,
                    AggregationArg::Last => resample::Aggregation::Last,
                    AggregationArg::Count => resample::Aggregation::Count,
                };
                let ds = resample::downsample(&series, *factor, agg);
                output::format_series(&ds, cli.output)
            }
            ResampleDirArg::Up => {
                let us = resample::upsample(&series, *factor);
                output::format_series(&us, cli.output)
            }
        },
        Command::Plot {
            plot_type,
            height,
            width,
            bins,
        } => {
            let values = series.values();
            match plot_type {
                PlotTypeArg::Sparkline => {
                    let sl = visualize::sparkline(&values);
                    if cli.output == OutputFormat::Json {
                        serde_json::json!({ "sparkline": sl }).to_string()
                    } else {
                        format!("{}  {}\n", sl, series.name)
                    }
                }
                PlotTypeArg::Line => visualize::line_chart(&values, *height, *width),
                PlotTypeArg::Histogram => visualize::histogram(&values, *bins),
                PlotTypeArg::Box => visualize::box_plot(&values),
            }
        }
        Command::Show => output::format_series(&series, cli.output),
    };

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    if handle.write_all(output.as_bytes()).is_err() {
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn load_series(input: &Option<String>, name: &str) -> Result<tseries::TimeSeries, String> {
    match input {
        Some(path) => reader::read_file(path, name).map_err(|e| e.to_string()),
        None => {
            let stdin = std::io::stdin();
            reader::read_series(stdin.lock(), name).map_err(|e| e.to_string())
        }
    }
}
