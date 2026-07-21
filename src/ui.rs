//! Terminal output. Colour is decoration, never load-bearing: everything
//! renders identically (minus ANSI codes) with `NO_COLOR` set or when stdout
//! is not a TTY.

use std::io::IsTerminal;

use owo_colors::OwoColorize;

/// Styled output helper. Construct once in `main` and pass by reference.
pub struct Ui {
    color: bool,
}

impl Ui {
    pub fn new() -> Ui {
        // NO_COLOR (any value) wins; otherwise colour only on a real terminal.
        let no_color = std::env::var_os("NO_COLOR").is_some();
        Ui {
            color: !no_color && std::io::stdout().is_terminal(),
        }
    }

    /// Section header, bold.
    pub fn header(&self, s: &str) -> String {
        if self.color {
            s.bold().to_string()
        } else {
            s.to_string()
        }
    }

    /// Success/confirmation line, green.
    pub fn ok(&self, s: &str) -> String {
        if self.color {
            s.green().to_string()
        } else {
            s.to_string()
        }
    }

    /// Warning/note line, yellow.
    pub fn warn(&self, s: &str) -> String {
        if self.color {
            s.yellow().to_string()
        } else {
            s.to_string()
        }
    }

    /// De-emphasized detail, dimmed.
    pub fn dim(&self, s: &str) -> String {
        if self.color {
            s.dimmed().to_string()
        } else {
            s.to_string()
        }
    }

    /// An error line for stderr: `error: <msg>`.
    pub fn error_line(&self, msg: &str) -> String {
        if self.color {
            format!("{} {msg}", "error:".red().bold())
        } else {
            format!("error: {msg}")
        }
    }

    /// A spinner for long operations (registry fetch). `None` when stdout is
    /// not a TTY — background/CI runs get a plain line from the caller instead.
    pub fn spinner(&self, msg: &str) -> Option<indicatif::ProgressBar> {
        if !self.color {
            return None;
        }
        let pb = indicatif::ProgressBar::new_spinner();
        pb.set_message(msg.to_string());
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(pb)
    }

    /// Render a unified diff of `old` → `new`, +green/−red when colour is on.
    pub fn print_diff(&self, old: &str, new: &str) {
        let diff = similar::TextDiff::from_lines(old, new);
        for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
            println!("    {}", self.dim(&hunk.header().to_string()));
            for change in hunk.iter_changes() {
                let line = change.value().trim_end_matches('\n');
                let rendered = match change.tag() {
                    similar::ChangeTag::Delete => {
                        let s = format!("-{line}");
                        if self.color {
                            s.red().to_string()
                        } else {
                            s
                        }
                    }
                    similar::ChangeTag::Insert => {
                        let s = format!("+{line}");
                        if self.color {
                            s.green().to_string()
                        } else {
                            s
                        }
                    }
                    similar::ChangeTag::Equal => format!(" {line}"),
                };
                println!("    {rendered}");
            }
        }
    }

    /// Whether prompts can be interactive (stdin+stdout are TTYs).
    pub fn interactive(&self) -> bool {
        std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
    }
}
