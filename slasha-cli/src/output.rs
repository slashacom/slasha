use std::time::Duration;

use anyhow::Result;
use colored::Colorize;
use comfy_table::{Cell, ContentArrangement, Table, presets::UTF8_FULL_CONDENSED};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum OutputMode {
    Human,
    Json,
}

impl OutputMode {
    pub fn is_json(&self) -> bool {
        matches!(self, OutputMode::Json)
    }
}

pub fn cli_success(msg: impl std::fmt::Display) {
    println!("{} {}", "SUCCESS:".green(), msg);
}

pub fn cli_error(msg: impl std::fmt::Display) {
    eprintln!("{} {}", "ERROR:".red(), msg);
}

pub fn cli_info(msg: impl std::fmt::Display) {
    println!("{}", msg);
}

pub fn cli_section(msg: impl std::fmt::Display) {
    println!("\n{}", msg.to_string().bold());
}

pub fn cli_label(key: impl std::fmt::Display, val: impl std::fmt::Display) {
    println!("  {} {}", format!("{key}:").dimmed(), val);
}

pub fn print_table(headers: &[&str], rows: Vec<Vec<String>>) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(headers.iter().map(Cell::new));

    for row in rows {
        table.add_row(row);
    }

    println!("{table}");
}

pub fn print_json<T: Serialize>(value: &T) -> Result<()> {
    match serde_json::to_string_pretty(value) {
        Ok(s) => {
            println!("{s}");
            Ok(())
        }
        Err(e) => {
            eprintln!("{} Failed to serialize JSON: {}", "ERROR:".red(), e);
            Err(e.into())
        }
    }
}

pub fn output<T, F>(output_mode: OutputMode, data: &T, human: F) -> Result<()>
where
    T: Serialize,
    F: FnOnce(),
{
    if output_mode.is_json() {
        print_json(data)?;
    } else {
        human();
    }

    Ok(())
}

pub fn confirm_action(output_mode: OutputMode, yes: bool, message: &str) -> Result<bool> {
    if yes || output_mode.is_json() {
        return Ok(true);
    }

    let confirmed = inquire::Confirm::new(message)
        .with_default(false)
        .prompt()?;

    if !confirmed {
        cli_info("Aborted.");
        return Ok(false);
    }

    Ok(true)
}

pub fn spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(80));

    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["|", "/", "-", "\\"])
            .template("{spinner:.cyan} {msg}")
            .expect("valid template"),
    );

    pb.set_message(msg.to_string());
    pb
}
