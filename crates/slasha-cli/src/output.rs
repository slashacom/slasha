use std::time::Duration;

use anyhow::Result;
use colored::Colorize;
use comfy_table::{Cell, ContentArrangement, Table, presets::UTF8_FULL};
use indicatif::{ProgressBar, ProgressStyle};

/// Prints a green success message to stdout.
///
/// # Arguments
///
/// * `msg` - Message content to display.
pub fn cli_success(msg: impl std::fmt::Display) {
    println!("{} {}", "SUCCESS:".green(), msg);
}

/// Prints a red error message to stderr.
///
/// # Arguments
///
/// * `msg` - Error message content to display.
pub fn cli_error(msg: impl std::fmt::Display) {
    eprintln!("{} {}", "ERROR:".red(), msg);
}

/// Prints a plain informational message line to stdout.
///
/// # Arguments
///
/// * `msg` - Message content to display.
pub fn cli_info(msg: impl std::fmt::Display) {
    println!("{}", msg);
}

/// Prints a bold section header line preceded by a newline.
///
/// # Arguments
///
/// * `msg` - Header text to display.
pub fn cli_section(msg: impl std::fmt::Display) {
    println!("\n{}", msg.to_string().bold());
}

/// Prints an indented key-value pair with a dimmed key label.
///
/// # Arguments
///
/// * `key` - Property key label.
/// * `val` - Property value text.
pub fn cli_label(key: impl std::fmt::Display, val: impl std::fmt::Display) {
    println!("  {} {}", format!("{key}:").dimmed(), val);
}

/// Renders a dynamic UTF-8 table with column headers to stdout.
///
/// # Arguments
///
/// * `headers` - Column header labels.
/// * `rows` - Matrix of table row string cells.
pub fn print_table(headers: &[&str], rows: Vec<Vec<String>>) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(headers.iter().map(Cell::new));

    for row in rows {
        table.add_row(row);
    }

    println!("{table}");
}

/// Prompts for interactive user confirmation unless pre-confirmed via flag.
///
/// # Arguments
///
/// * `yes` - Bypasses prompt if `true`.
/// * `message` - Confirmation question prompt text.
///
/// # Returns
///
/// `true` if action is confirmed, or `false` if canceled.
pub fn confirm_action(yes: bool, message: &str) -> Result<bool> {
    if yes {
        return Ok(true);
    }

    if !std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        anyhow::bail!(
            "Cannot prompt for confirmation in non-interactive environment. Use --yes flag."
        );
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

/// RAII drop guard that automatically finishes and clears a progress spinner upon drop.
pub struct SpinnerGuard {
    pb: Option<ProgressBar>,
}

impl Drop for SpinnerGuard {
    fn drop(&mut self) {
        if let Some(pb) = self.pb.take() {
            pb.finish_and_clear();
        }
    }
}

/// Spawns a cyan steady-tick spinner guard when attached to an interactive terminal.
///
/// # Arguments
///
/// * `msg` - Initial spinner status message.
///
/// # Returns
///
/// A [`SpinnerGuard`] that clears the spinner on drop.
pub fn spinner(msg: &str) -> SpinnerGuard {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(80));

    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["|", "/", "-", "\\"])
            .template("{spinner:.cyan} {msg}")
            .expect("valid template"),
    );

    pb.set_message(msg.to_string());
    SpinnerGuard { pb: Some(pb) }
}
