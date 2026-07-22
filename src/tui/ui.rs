// src/tui/ui.rs
//! UI rendering logic for the TUI

use crate::tui::app::TuiApp;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

pub fn render(f: &mut Frame, app: &mut TuiApp) {
    if app.show_logs {
        render_logs_panel(f, app, f.area());
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(90), Constraint::Percentage(10)])
            .split(f.area());

        render_message_list(f, app, chunks[0]);
        render_input_box(f, app, chunks[1]);
    }

    if app.show_help {
        render_help_overlay(f, app);
    }
}

fn render_message_list(f: &mut Frame, app: &mut TuiApp, area: Rect) {
    let messages = app.get_messages();
    let inner_width = area.width.saturating_sub(2) as usize;
    let visible_height = area.height.saturating_sub(2) as usize;
    let mut all_lines = Vec::new();

    for msg in &messages {
        if !app.show_raw && msg.starts_with("[Raw]") {
            continue;
        }

        let style = get_message_style(msg);
        for line_text in wrap_text(msg, inner_width) {
            all_lines.push((line_text, style));
        }
    }

    let total_lines = all_lines.len();
    let start_line = if app.auto_scroll {
        total_lines.saturating_sub(visible_height)
    } else {
        total_lines.saturating_sub(visible_height + app.scroll_offset)
    };

    let start_line = app.set_rendered_lines(
        all_lines.iter().map(|(text, _)| text.clone()).collect(),
        start_line,
        visible_height,
    );

    let visible_lines = all_lines
        .into_iter()
        .enumerate()
        .skip(start_line)
        .take(visible_height)
        .map(|(idx, (line_text, style))| {
            Line::from(Span::styled(
                line_text,
                style_for_line(app, idx, style, Color::Blue),
            ))
        })
        .collect::<Vec<_>>();

    let scroll_indicator = if app.visual_mode {
        "VISUAL | j/k move | g/G jump | y copy | Esc cancel"
    } else if app.pane_cursor().is_some() {
        "Up/Down history | PgUp/Dn scroll | Ctrl+Y select"
    } else if app.auto_scroll {
        "Auto-scroll"
    } else {
        "Paused - PgUp/Dn to scroll"
    };

    let online_count = app.get_online_count();
    let online_display = if online_count > 0 {
        format!(" | Online: {}", online_count)
    } else {
        String::new()
    };

    let raw_indicator = if app.show_raw { "Raw:ON" } else { "Raw:OFF" };
    let title = format!(
        " Room {}{} | {} | {} ",
        app.room_id, online_display, scroll_indicator, raw_indicator
    );

    let paragraph = Paragraph::new(visible_lines)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn get_message_style(msg: &str) -> Style {
    if msg.starts_with("[Danmu]") {
        Style::default().fg(Color::Cyan)
    } else if msg.starts_with("[Gift]") {
        Style::default().fg(Color::Yellow)
    } else if msg.starts_with("[Raw]") {
        Style::default().fg(Color::Magenta)
    } else if msg.starts_with("[Unsupported") {
        Style::default().fg(Color::DarkGray)
    } else if msg.starts_with("[System]") {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    }
}

fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for ch in text.chars() {
        let char_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);

        if current_width + char_width > max_width && !current_line.is_empty() {
            lines.push(current_line);
            current_line = String::new();
            current_width = 0;
        }

        current_line.push(ch);
        current_width += char_width;
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

fn render_logs_panel(f: &mut Frame, app: &mut TuiApp, area: Rect) {
    let logs = app.get_log_messages();
    let inner_width = area.width.saturating_sub(2) as usize;
    let visible_height = area.height.saturating_sub(2) as usize;
    let mut all_lines = Vec::new();

    for log_msg in &logs {
        let style = get_log_style(log_msg);
        for line_text in wrap_text(log_msg, inner_width) {
            all_lines.push((line_text, style));
        }
    }

    let total_lines = all_lines.len();
    let start_line = if app.log_auto_scroll {
        total_lines.saturating_sub(visible_height)
    } else {
        total_lines.saturating_sub(visible_height + app.log_scroll_offset)
    };

    let start_line = app.set_rendered_lines(
        all_lines.iter().map(|(text, _)| text.clone()).collect(),
        start_line,
        visible_height,
    );

    let visible_lines = all_lines
        .into_iter()
        .enumerate()
        .skip(start_line)
        .take(visible_height)
        .map(|(idx, (line_text, style))| {
            Line::from(Span::styled(
                line_text,
                style_for_line(app, idx, style, Color::LightBlue),
            ))
        })
        .collect::<Vec<_>>();

    let scroll_indicator = if app.visual_mode {
        "VISUAL | j/k move | g/G jump | y copy | Esc cancel"
    } else if app.pane_cursor().is_some() {
        "CURSOR | Up/Down move | Ctrl+Y visual from cursor"
    } else if app.log_auto_scroll {
        "Auto-scroll"
    } else {
        "Paused"
    };

    let title = format!(
        " Logs ({} entries) | {} | Ctrl+Y: visual | Ctrl+H: help | Ctrl+L: close ",
        logs.len(),
        scroll_indicator
    );

    let paragraph = Paragraph::new(visible_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::LightBlue)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn get_log_style(msg: &str) -> Style {
    if msg.contains("[ERROR]") {
        Style::default().fg(Color::Red)
    } else if msg.contains("[WARN]") {
        Style::default().fg(Color::Yellow)
    } else if msg.contains("[INFO]") {
        Style::default().fg(Color::Green)
    } else if msg.contains("[DEBUG]") || msg.contains("[TRACE]") {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
    }
}

fn render_input_box(f: &mut Frame, app: &TuiApp, area: Rect) {
    let input_text = format!("> {}", app.input);

    let paragraph = Paragraph::new(input_text.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Input (Up/Down: history | Ctrl+Y: visual | Ctrl+H: help | Ctrl+C: exit) ")
                .border_style(Style::default().fg(Color::Green)),
        )
        .style(Style::default());

    f.render_widget(paragraph, area);

    let text_before_cursor: String = app.input.chars().take(app.cursor_position).collect();
    let display_width = text_before_cursor.width();
    let cursor_x = area.x + 1 + 2 + display_width as u16;
    let cursor_y = area.y + 1;

    if !app.visual_mode && cursor_x < area.x + area.width.saturating_sub(1) {
        f.set_cursor_position((cursor_x, cursor_y));
    }
}

fn render_help_overlay(f: &mut Frame, app: &TuiApp) {
    let area = centered_rect(72, 72, f.area());
    let lines = if app.show_logs {
        vec![
            Line::from("Key Map"),
            Line::from(""),
            Line::from("Ctrl+H   Toggle this help"),
            Line::from("Up/Down  Pick start line"),
            Line::from("Ctrl+Y   Enter visual mode from cursor"),
            Line::from("j/k      Move visual selection"),
            Line::from("g / G    Jump to top or bottom"),
            Line::from("y        Copy selected lines"),
            Line::from("Esc      Close help, cancel visual, or close logs"),
            Line::from("Up/Down  Scroll logs normally"),
            Line::from("PgUp/Dn  Scroll faster"),
            Line::from("Home/End Jump to top or bottom"),
            Line::from("Ctrl+C   Exit app"),
        ]
    } else {
        vec![
            Line::from("Key Map"),
            Line::from(""),
            Line::from("Enter    Send input"),
            Line::from("Up/Down  Browse sent-input history"),
            Line::from("Ctrl+H   Toggle this help"),
            Line::from("Ctrl+Y   Enter visual mode (scroll/select)"),
            Line::from("j/k      Move visual selection"),
            Line::from("g / G    Jump to top or bottom"),
            Line::from("y        Copy selected lines"),
            Line::from("Ctrl+R   Toggle raw messages"),
            Line::from("Ctrl+L   Toggle logs panel"),
            Line::from("PgUp/Dn  Scroll messages"),
            Line::from("Left/Right Move input cursor"),
            Line::from("Home/End Move input cursor"),
            Line::from("Ctrl+Home Jump to top"),
            Line::from("Ctrl+End Jump to bottom"),
            Line::from("Esc      Close help or cancel visual"),
            Line::from("Ctrl+C   Exit app"),
        ]
    };

    let title = if app.show_logs {
        " Help - Logs "
    } else {
        " Help - Messages "
    };

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn style_for_line(app: &TuiApp, idx: usize, base: Style, cursor_color: Color) -> Style {
    if let Some((start, end)) = app.visual_range() {
        if Some(idx) == app.visual_cursor() {
            return base.bg(cursor_color).fg(Color::Black);
        }

        if (start..=end).contains(&idx) {
            return base.bg(Color::DarkGray).fg(Color::White);
        }
    }

    if Some(idx) == app.pane_cursor() {
        return base.bg(Color::Rgb(40, 40, 40)).fg(Color::White);
    }

    base
}
