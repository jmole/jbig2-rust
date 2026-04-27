use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use plotters::coord::Shift;
use plotters::prelude::*;

use super::summary::{ProbeRecord, criterion_output_dir, estimates_path, parse_mean_ns};
use super::{DECODE_CASES, ENCODE_CASES};

fn tool_color(tool: &str) -> RGBColor {
    match tool {
        "rust" => RGBColor(0x1F, 0x77, 0xB4),
        "jbig2enc" | "jbig2dec" => RGBColor(0xFF, 0x7F, 0x0E),
        "t88" => RGBColor(0x8E, 0x44, 0xAD),
        _ => RGBColor(0x55, 0x55, 0x55),
    }
}

pub(crate) fn render_comparison_chart(
    records: &[ProbeRecord],
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    let mut data: BTreeMap<(&'static str, &'static str, &'static str), f64> = BTreeMap::new();
    for r in records {
        let est = estimates_path(r.side, r.tool, r.case);
        if let Some(ns) = parse_mean_ns(&est) {
            if ns > 0.0 && r.raw_bytes > 0 {
                let mib = (r.raw_bytes as f64) / (ns / 1e9) / (1024.0 * 1024.0);
                data.insert((r.side, r.case, r.tool), mib);
            }
        }
    }
    if data.is_empty() {
        return Ok(None);
    }

    let out_path = criterion_output_dir().join("reference_codec_chart.svg");
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    {
        let root = SVGBackend::new(&out_path, (1200, 900)).into_drawing_area();
        root.fill(&WHITE)?;
        let root = root.titled(
            "Reference codec throughput comparison (higher is better)",
            ("sans-serif", 26),
        )?;

        let (top, bot) = root.split_vertically(430);
        let decode_cases: Vec<&'static str> = DECODE_CASES.iter().map(|c| c.tag).collect();
        let encode_cases: Vec<&'static str> = ENCODE_CASES.iter().map(|c| c.tag).collect();
        let decode_tools: &[&str] = &["rust", "jbig2dec", "t88"];
        let encode_tools: &[&str] = &["rust", "jbig2enc", "t88"];

        draw_side_chart(&top, "Decode", "decode", &decode_cases, decode_tools, &data)?;
        draw_side_chart(&bot, "Encode", "encode", &encode_cases, encode_tools, &data)?;

        root.present()?;
    }

    Ok(Some(out_path))
}

fn draw_side_chart<DB>(
    area: &DrawingArea<DB, Shift>,
    title: &str,
    side: &'static str,
    cases: &[&'static str],
    tools: &[&str],
    data: &BTreeMap<(&'static str, &'static str, &'static str), f64>,
) -> Result<(), Box<dyn std::error::Error>>
where
    DB: DrawingBackend,
    DB::ErrorType: 'static,
{
    let present_tools: Vec<&str> = tools
        .iter()
        .copied()
        .filter(|t| cases.iter().any(|c| data.contains_key(&(side, *c, *t))))
        .collect();
    if present_tools.is_empty() {
        return Ok(());
    }

    let max_mib = cases
        .iter()
        .flat_map(|c| {
            present_tools
                .iter()
                .filter_map(|t| data.get(&(side, *c, *t)).copied())
        })
        .fold(0.0_f64, f64::max);
    let min_mib = cases
        .iter()
        .flat_map(|c| {
            present_tools
                .iter()
                .filter_map(|t| data.get(&(side, *c, *t)).copied())
        })
        .fold(f64::INFINITY, f64::min)
        .max(0.1);
    let y_min = (min_mib / 2.0).max(0.1);
    let y_max = (max_mib * 2.5).max(1.0);

    let mut chart = ChartBuilder::on(area)
        .caption(
            format!("{title} throughput (MiB/s, log scale)"),
            ("sans-serif", 22),
        )
        .margin(20)
        .x_label_area_size(50)
        .y_label_area_size(70)
        .right_y_label_area_size(10)
        .build_cartesian_2d(-0.5..(cases.len() as f64) - 0.5, (y_min..y_max).log_scale())?;

    chart
        .configure_mesh()
        .disable_x_mesh()
        .x_labels(cases.len())
        .x_label_formatter(&|x| {
            let i = x.round() as i64;
            if i >= 0 && (i as usize) < cases.len() {
                cases[i as usize].to_string()
            } else {
                String::new()
            }
        })
        .x_desc("case")
        .y_desc("MiB/s")
        .label_style(("sans-serif", 14))
        .axis_desc_style(("sans-serif", 14))
        .draw()?;

    let group_width: f64 = 0.82;
    let bar_width: f64 = group_width / present_tools.len() as f64;

    for (t_idx, tool) in present_tools.iter().enumerate() {
        let color = tool_color(tool);
        let offset = (t_idx as f64) * bar_width - group_width / 2.0 + bar_width / 2.0;
        let bars: Vec<(f64, f64, f64)> = cases
            .iter()
            .enumerate()
            .filter_map(|(c_idx, case)| {
                let mib = *data.get(&(side, *case, *tool))?;
                let x_center = (c_idx as f64) + offset;
                Some((x_center - bar_width / 2.0, x_center + bar_width / 2.0, mib))
            })
            .collect();

        let legend_color = color;
        chart
            .draw_series(
                bars.iter()
                    .map(|&(x0, x1, mib)| Rectangle::new([(x0, y_min), (x1, mib)], color.filled())),
            )?
            .label(*tool)
            .legend(move |(x, y)| {
                Rectangle::new([(x, y - 6), (x + 14, y + 6)], legend_color.filled())
            });

        for &(x0, x1, mib) in &bars {
            let x_mid = (x0 + x1) / 2.0;
            let label = if mib >= 100.0 {
                format!("{mib:.0}")
            } else if mib >= 10.0 {
                format!("{mib:.1}")
            } else {
                format!("{mib:.2}")
            };
            chart.draw_series(std::iter::once(Text::new(
                label,
                (x_mid, mib * 1.08),
                ("sans-serif", 12).into_font().color(&BLACK),
            )))?;
        }
    }

    chart
        .configure_series_labels()
        .background_style(WHITE.mix(0.9))
        .border_style(BLACK.mix(0.3))
        .label_font(("sans-serif", 14))
        .position(SeriesLabelPosition::UpperRight)
        .draw()?;

    Ok(())
}
