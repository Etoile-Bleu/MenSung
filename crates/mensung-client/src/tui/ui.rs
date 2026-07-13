//! Renders the current App state. Color rules match README.md: red for
//! danger, yellow for warning, green for no known interaction. Nothing here
//! mutates App; rendering is a pure function of state.

use mensung_domain::Severity;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use super::app::{App, Screen};

const TITLE: &str = " MenSung -- offline medication interaction checker ";
const HELP_INPUT: &str = "Tab/Up/Down: switch field  Enter: check  Esc: quit";
const HELP_CANDIDATES: &str = "Up/Down: select  Enter: confirm  Esc: back";
const HELP_DISMISS: &str = "Enter/Esc: back";

pub(crate) fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new(TITLE).block(Block::default().borders(Borders::ALL)),
        chunks[0],
    );

    match app.screen() {
        Screen::Input => draw_input(frame, app, chunks[1]),
        Screen::Candidates {
            field,
            candidates,
            selected,
        } => draw_candidates(
            frame,
            app.inputs()[*field].as_str(),
            candidates,
            *selected,
            chunks[1],
        ),
        Screen::NoMatch { field } => draw_message(
            frame,
            "Unknown drug",
            &format!(
                "No similar name was found for '{}' in the database.",
                app.inputs()[*field]
            ),
            Color::Yellow,
            chunks[1],
        ),
        Screen::Error(message) => draw_message(frame, "Error", message, Color::Red, chunks[1]),
        Screen::Results { interactions } => draw_results(frame, interactions, chunks[1]),
    }

    let help = match app.screen() {
        Screen::Input => HELP_INPUT,
        Screen::Candidates { .. } => HELP_CANDIDATES,
        Screen::NoMatch { .. } | Screen::Error(_) | Screen::Results { .. } => HELP_DISMISS,
    };
    frame.render_widget(Paragraph::new(help), chunks[2]);
}

fn draw_input(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3)])
        .split(area);

    for (i, row) in rows.iter().enumerate() {
        let label = format!("Drug {}", i + 1);
        let focused = app.focused() == i;
        let style = if focused {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .title(label)
            .border_style(if focused {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            });
        frame.render_widget(
            Paragraph::new(app.inputs()[i].as_str())
                .style(style)
                .block(block),
            *row,
        );
    }
}

fn draw_candidates(
    frame: &mut Frame,
    query: &str,
    candidates: &[mensung_core::Candidate],
    selected: usize,
    area: Rect,
) {
    let items: Vec<ListItem> = candidates
        .iter()
        .enumerate()
        .map(|(i, candidate)| {
            let line = format!(
                "{} ({:.1}%)",
                candidate.drug().name(),
                candidate.similarity() * 100.0
            );
            let style = if i == selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("Unknown drug '{query}', did you mean")),
    );
    frame.render_widget(list, area);
}

fn draw_message(frame: &mut Frame, title: &str, message: &str, color: Color, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(color));
    frame.render_widget(Paragraph::new(message).block(block), area);
}

fn severity_color(severity: Severity) -> Color {
    match severity {
        Severity::Contraindicated | Severity::HighRisk => Color::Red,
        Severity::Moderate | Severity::Minor | Severity::Unknown => Color::Yellow,
    }
}

fn draw_results(frame: &mut Frame, interactions: &[mensung_db::InteractionRecord], area: Rect) {
    if interactions.is_empty() {
        draw_message(
            frame,
            "Result",
            "No known interaction among the selected drugs.",
            Color::Green,
            area,
        );
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    for interaction in interactions {
        let color = severity_color(interaction.severity());
        lines.push(Line::from(Span::styled(
            format!("!!! {} INTERACTION !!!", interaction.severity()),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(interaction.description().to_string()));
        lines.push(Line::from(format!(
            "Evidence: {} ({})",
            interaction.evidence(),
            interaction.source()
        )));
        lines.push(Line::from(""));
    }

    let block = Block::default().borders(Borders::ALL).title("Interactions");
    frame.render_widget(Paragraph::new(lines).block(block), area);
}
