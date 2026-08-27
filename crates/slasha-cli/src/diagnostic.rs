use anyhow::Result;
use git_version::git_version;

enum DiagnosticEntry {
    Text(String),
    List(Vec<DiagnosticEntry>),
}

struct DiagnosticSection<'a> {
    title: &'a str,
    entry: DiagnosticEntry,
}

/// Diagnostic report containing system and dependency telemetry details.
pub struct DiagnosticReport<'a> {
    sections: Vec<DiagnosticSection<'a>>,
}

impl<'a> DiagnosticReport<'a> {
    /// Generates a diagnostic report collecting software, OS, dependency, and build environment metadata.
    ///
    /// # Returns
    ///
    /// A new [`DiagnosticReport`] instance.
    pub fn generate() -> Result<DiagnosticReport<'a>> {
        let mut sections = vec![];

        sections.push(DiagnosticSection {
            title: "Software version",
            entry: DiagnosticEntry::List(vec![
                DiagnosticEntry::Text(format!(
                    "{} {} ({})",
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION"),
                    git_version!(fallback = "")
                )),
                DiagnosticEntry::Text(format!("Build timestamp: {}", env!("BUILD_TIMESTAMP"))),
            ]),
        });

        sections.push(DiagnosticSection {
            title: "Dependencies",
            entry: DiagnosticEntry::List(vec![
                DiagnosticEntry::Text(format!(
                    "Docker: {}",
                    check_command_version("docker", &["--version"])
                )),
                DiagnosticEntry::Text(format!(
                    "Railpack: {}",
                    check_command_version("railpack", &["--version"])
                )),
            ]),
        });

        sections.push(DiagnosticSection {
            title: "Operating system",
            entry: DiagnosticEntry::List(vec![
                DiagnosticEntry::Text(format!(
                    "OS: {}",
                    sysinfo::System::long_os_version().unwrap_or_else(|| "Unknown".to_owned()),
                )),
                DiagnosticEntry::Text(format!(
                    "Kernel: {}",
                    sysinfo::System::kernel_version().unwrap_or_else(|| "Unknown".to_owned()),
                )),
            ]),
        });

        #[cfg(target_family = "unix")]
        {
            if let Ok(shell) = std::env::var("SHELL") {
                sections.push(DiagnosticSection {
                    title: "Shell",
                    entry: DiagnosticEntry::Text(shell),
                });
            }
        }

        sections.push(DiagnosticSection {
            title: "Compile time information",
            entry: DiagnosticEntry::List(vec![
                DiagnosticEntry::Text(format!("Profile: {}", env!("PROFILE"))),
                DiagnosticEntry::Text(format!("Target triple: {}", env!("TARGET"))),
                DiagnosticEntry::Text(format!("Family: {}", env!("CARGO_CFG_TARGET_FAMILY"))),
                DiagnosticEntry::Text(format!("OS: {}", env!("CARGO_CFG_TARGET_OS"))),
                DiagnosticEntry::Text(format!("Architecture: {}", env!("CARGO_CFG_TARGET_ARCH"))),
                DiagnosticEntry::Text(format!(
                    "Pointer width: {}",
                    env!("CARGO_CFG_TARGET_POINTER_WIDTH")
                )),
                DiagnosticEntry::Text(format!("Endian: {}", env!("CARGO_CFG_TARGET_ENDIAN"))),
                DiagnosticEntry::Text(format!(
                    "CPU features: {}",
                    env!("CARGO_CFG_TARGET_FEATURE")
                )),
                DiagnosticEntry::Text(format!("Host: {}", env!("HOST"))),
            ]),
        });

        Ok(DiagnosticReport { sections })
    }

    /// Prints the formatted diagnostic report sections in Markdown format to stdout.
    pub fn print(&self) -> Result<()> {
        let mut output = String::new();

        for section in &self.sections {
            output += &format_section(section.title);
            output += &format_entry(&section.entry);
            output += "\n";
        }

        println!("{}", output.trim_end());
        Ok(())
    }
}

fn check_command_version(cmd: &str, args: &[&str]) -> String {
    match std::process::Command::new(cmd).args(args).output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if version.is_empty() {
                "Installed".to_string()
            } else {
                format!("Installed ({version})")
            }
        }
        _ => "Not installed".to_string(),
    }
}

fn format_section(title: &str) -> String {
    format!("#### {}\n\n", title)
}

fn format_entry(entry: &DiagnosticEntry) -> String {
    match entry {
        DiagnosticEntry::Text(content) => format!("{}\n", content),
        DiagnosticEntry::List(entries) => {
            entries
                .iter()
                .map(|e| format!("- {}", format_entry(e).trim_end()))
                .collect::<Vec<_>>()
                .join("\n")
                + "\n"
        }
    }
}
