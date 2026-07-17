//! ASCII visualization: sparklines, line charts, and histograms.

use crate::stats;

/// Generate a Unicode sparkline from values.
/// Returns a string like "▁▂▃▄▅▆▇█".
pub fn sparkline(values: &[f64]) -> String {
    if values.is_empty() {
        return String::new();
    }
    let blocks = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;
    if range.abs() < f64::EPSILON {
        return blocks[3].to_string().repeat(values.len());
    }
    values
        .iter()
        .map(|v| {
            let normalized = (v - min) / range;
            let idx = (normalized * 7.0).round() as usize;
            blocks[idx.min(7)]
        })
        .collect()
}

/// Generate an ASCII line chart with the given height (in rows).
pub fn line_chart(values: &[f64], height: usize, width: usize) -> String {
    if values.is_empty() || height == 0 || width == 0 {
        return String::new();
    }

    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;
    if range.abs() < f64::EPSILON {
        // Flat line
        let row = "─".repeat(width.min(60));
        return format!("{min:.2} │{row}\n");
    }

    // Sample values to fit width
    let step = (values.len() as f64 / width as f64).max(1.0);
    let sampled: Vec<f64> = (0..width)
        .map(|i| {
            let idx = (i as f64 * step) as usize;
            values.get(idx).copied().unwrap_or(values[values.len() - 1])
        })
        .collect();

    let w = sampled.len();
    let mut grid = vec![vec![' '; w]; height];

    for (col, &v) in sampled.iter().enumerate() {
        let normalized = (v - min) / range;
        let row = ((1.0 - normalized) * (height - 1) as f64).round() as usize;
        grid[row][col] = '●';

        // Draw vertical line from bottom to the point
        grid[(row + 1)..].iter_mut().for_each(|r| r[col] = '│');
    }

    let mut output = String::new();
    for (i, row) in grid.iter().enumerate() {
        let label_val = max - (i as f64 / (height - 1).max(1) as f64) * range;
        output.push_str(&format!("{:>8.2} │", label_val));
        output.push_str(&row.iter().collect::<String>());
        output.push('\n');
    }
    // X-axis
    output.push_str(&format!("{:>8} └{}", "", "─".repeat(w)));
    output.push('\n');

    output
}

/// Generate an ASCII histogram.
pub fn histogram(values: &[f64], bins: usize) -> String {
    if values.is_empty() || bins == 0 {
        return String::new();
    }

    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;
    if range.abs() < f64::EPSILON {
        return format!(
            "[{min:.2} - {max:.2}] {count}",
            min = min,
            max = max,
            count = values.len()
        );
    }

    let bin_width = range / bins as f64;
    let mut counts = vec![0usize; bins];

    for &v in values {
        let bin_idx = ((v - min) / bin_width).floor() as usize;
        let bin_idx = bin_idx.min(bins - 1);
        counts[bin_idx] += 1;
    }

    let max_count = *counts.iter().max().unwrap_or(&1);
    let bar_width = 40;

    let mut output = String::new();
    for (i, &count) in counts.iter().enumerate() {
        let lo = min + i as f64 * bin_width;
        let hi = lo + bin_width;
        let bar_len = if max_count > 0 {
            (count as f64 / max_count as f64 * bar_width as f64).round() as usize
        } else {
            0
        };
        let bar = "█".repeat(bar_len);
        output.push_str(&format!(
            "[{:>10.2} - {:<10.2}] {:>5} │{}\n",
            lo, hi, count, bar
        ));
    }
    output
}

/// Generate a box plot (5-number summary) as ASCII.
pub fn box_plot(values: &[f64]) -> String {
    if values.is_empty() {
        return String::new();
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let min = sorted[0];
    let q1 = stats::percentile_sorted(&sorted, 25.0);
    let med = stats::percentile_sorted(&sorted, 50.0);
    let q3 = stats::percentile_sorted(&sorted, 75.0);
    let max = sorted[sorted.len() - 1];

    let width = 50;
    let range = max - min;
    if range.abs() < f64::EPSILON {
        return format!("  {min:.2} |─── {med:.2} ───| {max:.2}");
    }

    let scale = |v: f64| ((v - min) / range * width as f64).round() as usize;
    let min_pos = scale(min);
    let q1_pos = scale(q1);
    let med_pos = scale(med);
    let q3_pos = scale(q3);
    let max_pos = scale(max);

    let mut line = vec![' '; width + 1];

    // Whiskers
    for (i, cell) in line.iter_mut().enumerate() {
        if i >= min_pos && i <= max_pos {
            *cell = '─';
        }
    }
    // Box (Q1 to Q3)
    for (i, cell) in line.iter_mut().enumerate() {
        if i >= q1_pos && i <= q3_pos {
            *cell = '█';
        }
    }
    // Median marker
    if med_pos <= width {
        line[med_pos] = '│';
    }

    let plot: String = line.iter().collect();
    format!("{min:>10.2} ─┤{plot}├─ {max:.2}\n             │  Q1={q1:.2}  Med={med:.2}  Q3={q3:.2}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparkline_basic() {
        let vals = [1.0, 2.0, 3.0, 4.0, 5.0];
        let sl = sparkline(&vals);
        assert!(!sl.is_empty());
        assert_eq!(sl.chars().count(), 5);
    }

    #[test]
    fn test_sparkline_empty() {
        assert_eq!(sparkline(&[]), "");
    }

    #[test]
    fn test_sparkline_constant() {
        let sl = sparkline(&[5.0, 5.0, 5.0]);
        assert_eq!(sl, "▄▄▄");
    }

    #[test]
    fn test_line_chart() {
        let vals: Vec<f64> = (0..20).map(|i| i as f64).collect();
        let chart = line_chart(&vals, 5, 20);
        assert!(!chart.is_empty());
        assert!(chart.contains('●'));
    }

    #[test]
    fn test_histogram() {
        let vals: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let h = histogram(&vals, 10);
        assert!(!h.is_empty());
        assert!(h.contains('█'));
    }

    #[test]
    fn test_box_plot() {
        let vals: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let bp = box_plot(&vals);
        assert!(!bp.is_empty());
        assert!(bp.contains("Q1="));
        assert!(bp.contains("Med="));
        assert!(bp.contains("Q3="));
    }

    #[test]
    fn test_histogram_empty() {
        assert_eq!(histogram(&[], 10), "");
    }
}
