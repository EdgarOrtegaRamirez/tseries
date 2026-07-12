//! Output formatting: text, JSON, and CSV.

use crate::anomaly::Anomaly;
use crate::forecast::Forecast;
use crate::seasonality::SeasonalityResult;
use crate::stats::Stats;
use crate::trend::TrendResult;
use crate::TimeSeries;

/// Output format selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
    Csv,
}

impl OutputFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputFormat::Text => "text",
            OutputFormat::Json => "json",
            OutputFormat::Csv => "csv",
        }
    }
}

// ── Stats output ──

pub fn format_stats(stats: &Stats, name: &str, format: OutputFormat) -> String {
    match format {
        OutputFormat::Text => format_stats_text(stats, name),
        OutputFormat::Json => serde_json::to_string_pretty(stats).unwrap_or_default(),
        OutputFormat::Csv => format_stats_csv(stats, name),
    }
}

fn format_stats_text(stats: &Stats, name: &str) -> String {
    let mut s = String::new();
    s.push_str(&format!("═══ Statistics: {name} ═══\n\n"));
    s.push_str(&format!("  Count:      {}\n", stats.count));
    s.push_str(&format!("  Mean:       {:.4}\n", stats.mean));
    s.push_str(&format!("  Median:     {:.4}\n", stats.median));
    s.push_str(&format!("  Std Dev:    {:.4}\n", stats.stddev));
    s.push_str(&format!("  Variance:   {:.4}\n", stats.variance));
    s.push_str(&format!("  Min:        {:.4}\n", stats.min));
    s.push_str(&format!("  Max:        {:.4}\n", stats.max));
    s.push_str(&format!("  Range:      {:.4}\n", stats.range));
    s.push_str(&format!("  Sum:        {:.4}\n", stats.sum));
    s.push_str(&format!("  IQR:        {:.4}\n", stats.iqr));
    s.push_str(&format!("  CV:         {:.4}\n", stats.cv));
    s.push_str(&format!("  Skewness:   {:.4}\n", stats.skewness));
    s.push_str(&format!("  Kurtosis:   {:.4}\n", stats.kurtosis));
    s.push_str("\n  Percentiles:\n");
    s.push_str(&format!("    P25:  {:.4}\n", stats.p25));
    s.push_str(&format!("    P50:  {:.4}\n", stats.p50));
    s.push_str(&format!("    P75:  {:.4}\n", stats.p75));
    s.push_str(&format!("    P90:  {:.4}\n", stats.p90));
    s.push_str(&format!("    P95:  {:.4}\n", stats.p95));
    s.push_str(&format!("    P99:  {:.4}\n", stats.p99));
    s
}

fn format_stats_csv(stats: &Stats, name: &str) -> String {
    let mut s = String::new();
    s.push_str("metric,value\n");
    s.push_str(&format!("name,{name}\n"));
    s.push_str(&format!("count,{}\n", stats.count));
    s.push_str(&format!("mean,{:.6}\n", stats.mean));
    s.push_str(&format!("median,{:.6}\n", stats.median));
    s.push_str(&format!("stddev,{:.6}\n", stats.stddev));
    s.push_str(&format!("variance,{:.6}\n", stats.variance));
    s.push_str(&format!("min,{:.6}\n", stats.min));
    s.push_str(&format!("max,{:.6}\n", stats.max));
    s.push_str(&format!("range,{:.6}\n", stats.range));
    s.push_str(&format!("sum,{:.6}\n", stats.sum));
    s.push_str(&format!("iqr,{:.6}\n", stats.iqr));
    s.push_str(&format!("cv,{:.6}\n", stats.cv));
    s.push_str(&format!("skewness,{:.6}\n", stats.skewness));
    s.push_str(&format!("kurtosis,{:.6}\n", stats.kurtosis));
    s.push_str(&format!("p25,{:.6}\n", stats.p25));
    s.push_str(&format!("p50,{:.6}\n", stats.p50));
    s.push_str(&format!("p75,{:.6}\n", stats.p75));
    s.push_str(&format!("p90,{:.6}\n", stats.p90));
    s.push_str(&format!("p95,{:.6}\n", stats.p95));
    s.push_str(&format!("p99,{:.6}\n", stats.p99));
    s
}

// ── Anomaly output ──

pub fn format_anomalies(anomalies: &[Anomaly], format: OutputFormat) -> String {
    if anomalies.is_empty() {
        return match format {
            OutputFormat::Text => "No anomalies detected.\n".into(),
            OutputFormat::Json => "[]".into(),
            OutputFormat::Csv => "timestamp,value,method,score,threshold\n".into(),
        };
    }
    match format {
        OutputFormat::Text => {
            let mut s = format!("═══ Anomalies ({}) ═══\n\n", anomalies.len());
            s.push_str("  Timestamp           Value       Method          Score    Threshold\n");
            s.push_str("  ──────────────────── ─────────── ─────────────── ──────── ──────────\n");
            for a in anomalies {
                s.push_str(&format!(
                    "  {:<20} {:>10.4} {:<15} {:>8.4} {:>10.4}\n",
                    a.timestamp,
                    a.value,
                    a.method.as_str(),
                    a.score,
                    a.threshold
                ));
            }
            s
        }
        OutputFormat::Json => serde_json::to_string_pretty(anomalies).unwrap_or_default(),
        OutputFormat::Csv => {
            let mut s = String::from("timestamp,value,method,score,threshold\n");
            for a in anomalies {
                s.push_str(&format!(
                    "{},{:.6},{},{:.6},{:.6}\n",
                    a.timestamp,
                    a.value,
                    a.method.as_str(),
                    a.score,
                    a.threshold
                ));
            }
            s
        }
    }
}

// ── Trend output ──

pub fn format_trend(trend: &TrendResult, format: OutputFormat) -> String {
    match format {
        OutputFormat::Text => {
            let mut s = String::from("═══ Trend Analysis ═══\n\n");
            s.push_str(&format!("  Direction:    {}\n", trend.direction.as_str()));
            s.push_str(&format!("  Slope:        {:.6}\n", trend.slope));
            s.push_str(&format!("  Intercept:    {:.6}\n", trend.intercept));
            s.push_str(&format!("  R²:           {:.6}\n", trend.r_squared));
            s.push_str(&format!("  Correlation:  {:.6}\n", trend.correlation));
            if let Some(gr) = trend.growth_rate {
                s.push_str(&format!("  Growth Rate:  {:.4}%\n", gr));
            }
            s
        }
        OutputFormat::Json => serde_json::to_string_pretty(trend).unwrap_or_default(),
        OutputFormat::Csv => {
            format!(
                "metric,value\nslope,{:.6}\nintercept,{:.6}\nr_squared,{:.6}\ncorrelation,{:.6}\n",
                trend.slope, trend.intercept, trend.r_squared, trend.correlation
            )
        }
    }
}

// ── Forecast output ──

pub fn format_forecast(forecast: &Forecast, format: OutputFormat) -> String {
    match format {
        OutputFormat::Text => {
            let mut s = format!("═══ Forecast ({}) ═══\n\n", forecast.method);
            s.push_str(&format!("  MAE:  {:.6}\n", forecast.mae));
            s.push_str(&format!("  RMSE: {:.6}\n\n", forecast.rmse));
            s.push_str("  Predictions:\n");
            for (i, p) in forecast.predictions.iter().enumerate() {
                s.push_str(&format!("    t+{:>3}: {:.6}\n", i + 1, p));
            }
            s
        }
        OutputFormat::Json => serde_json::to_string_pretty(forecast).unwrap_or_default(),
        OutputFormat::Csv => {
            let mut s = format!("step,prediction\n");
            for (i, p) in forecast.predictions.iter().enumerate() {
                s.push_str(&format!("t+{},{:.6}\n", i + 1, p));
            }
            s
        }
    }
}

// ── Seasonality output ──

pub fn format_seasonality(result: &SeasonalityResult, format: OutputFormat) -> String {
    match format {
        OutputFormat::Text => {
            let mut s = String::from("═══ Seasonality Analysis ═══\n\n");
            if let Some(period) = result.period {
                s.push_str(&format!("  Detected Period: {}\n", period));
                s.push_str(&format!("  Strength:         {:.4}\n", result.strength));
            } else {
                s.push_str("  No significant seasonality detected.\n");
                s.push_str(&format!(
                    "  Max ACF: {:.4} at lag {}\n",
                    result.max_acf, result.max_lag
                ));
            }
            s.push_str("\n  Autocorrelation:\n");
            for (lag, acf) in result.autocorrelations.iter().enumerate() {
                let bar = "█".repeat((acf.abs() * 30.0).round() as usize);
                let sign = if *acf >= 0.0 { "+" } else { "-" };
                s.push_str(&format!("  lag {:>3}: {}{} {:.4}\n", lag, sign, bar, acf));
            }
            s
        }
        OutputFormat::Json => serde_json::to_string_pretty(result).unwrap_or_default(),
        OutputFormat::Csv => {
            let mut s = String::from("lag,autocorrelation\n");
            for (lag, acf) in result.autocorrelations.iter().enumerate() {
                s.push_str(&format!("{},{:.6}\n", lag, acf));
            }
            s
        }
    }
}

// ── Series output ──

pub fn format_series(series: &TimeSeries, format: OutputFormat) -> String {
    match format {
        OutputFormat::Text => {
            let mut s = format!(
                "═══ Series: {} ({} points) ═══\n\n",
                series.name,
                series.len()
            );
            s.push_str("  Timestamp           Value\n");
            s.push_str("  ──────────────────── ───────────\n");
            for p in &series.points {
                s.push_str(&format!("  {:<20} {:>10.4}\n", p.timestamp, p.value));
            }
            s
        }
        OutputFormat::Json => serde_json::to_string_pretty(series).unwrap_or_default(),
        OutputFormat::Csv => {
            let mut s = String::from("timestamp,value\n");
            for p in &series.points {
                s.push_str(&format!("{},{:.6}\n", p.timestamp, p.value));
            }
            s
        }
    }
}
