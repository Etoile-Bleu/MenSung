//! Terminal color helpers shared by the CLI's interaction/info output
//! (`cli.rs`) and the dataset installer's progress messages (`data.rs`,
//! `dataset_download.rs`), so both present the same red/yellow/green
//! severity convention the TUI already uses. Colors are only ever applied
//! when the destination stream is a real terminal, so piped or redirected
//! output stays plain text.

use std::io::IsTerminal as _;

use crossterm::style::Stylize;

#[derive(Clone, Copy)]
pub(crate) enum Tone {
    Danger,
    Warning,
    Ok,
    Dim,
    Bold,
}

pub(crate) fn styled_out(text: &str, tone: Tone) -> String {
    styled(text, tone, std::io::stdout().is_terminal())
}

pub(crate) fn styled_err(text: &str, tone: Tone) -> String {
    styled(text, tone, std::io::stderr().is_terminal())
}

fn styled(text: &str, tone: Tone, is_terminal: bool) -> String {
    if !is_terminal {
        return text.to_string();
    }
    match tone {
        Tone::Danger => text.red().bold().to_string(),
        Tone::Warning => text.yellow().to_string(),
        Tone::Ok => text.green().to_string(),
        Tone::Dim => text.dark_grey().to_string(),
        Tone::Bold => text.bold().to_string(),
    }
}
