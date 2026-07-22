// src/tui/event.rs
//! Event handling and main TUI loop

use crate::tui::app::TuiApp;
use crate::tui::ui;
use arboard::Clipboard;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;

/// Run the TUI application
pub fn run_tui<F>(mut app: TuiApp, mut on_message: F) -> io::Result<()>
where
    F: FnMut(String),
{
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &mut app, &mut on_message);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app<F>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut TuiApp,
    on_message: &mut F,
) -> io::Result<()>
where
    F: FnMut(String),
{
    let mut needs_redraw = true;
    let mut clipboard = Clipboard::new().ok();
    let mut last_message_count = app.message_count();
    let mut last_log_count = app.log_message_count();
    let mut last_online_count = app.get_online_count();

    loop {
        let message_count = app.message_count();
        let log_count = app.log_message_count();
        let online_count = app.get_online_count();

        if !app.visual_mode
            && (message_count != last_message_count
                || log_count != last_log_count
                || online_count != last_online_count)
        {
            needs_redraw = true;
        }

        last_message_count = message_count;
        last_log_count = log_count;
        last_online_count = online_count;

        if needs_redraw {
            terminal.draw(|f| ui::render(f, app))?;
            needs_redraw = false;
        }

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.quit();
                        needs_redraw = true;
                    }
                    KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.toggle_visual_mode();
                        needs_redraw = true;
                    }
                    KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if !app.visual_mode {
                            app.toggle_help();
                            needs_redraw = true;
                        }
                    }
                    KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if !app.visual_mode {
                            app.toggle_show_raw();
                            needs_redraw = true;
                        }
                    }
                    KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if !app.visual_mode {
                            app.toggle_show_logs();
                            needs_redraw = true;
                        }
                    }
                    KeyCode::Esc => {
                        // Esc only dismisses overlays; use Ctrl+C to quit the app.
                        if app.visual_mode {
                            app.exit_visual_mode();
                        } else if app.show_help {
                            app.show_help = false;
                        } else if app.show_logs {
                            app.toggle_show_logs();
                        }

                        needs_redraw = true;
                    }

                    _ if app.show_help => {}

                    _ if app.visual_mode => {
                        match key.code {
                            KeyCode::Char('k') | KeyCode::Up => app.visual_up(1),
                            KeyCode::Char('j') | KeyCode::Down => app.visual_down(1),
                            KeyCode::PageUp => app.visual_up(10),
                            KeyCode::PageDown => app.visual_down(10),
                            KeyCode::Char('g') | KeyCode::Home => app.visual_top(),
                            KeyCode::Char('G') | KeyCode::End => app.visual_bottom(),
                            KeyCode::Char('y') => {
                                copy_selection(app, clipboard.as_mut())?;
                                app.exit_visual_mode();
                            }
                            _ => {}
                        }

                        needs_redraw = true;
                    }

                    _ if app.show_logs => match key.code {
                        KeyCode::Up => {
                            app.pane_up(1);
                            needs_redraw = true;
                        }
                        KeyCode::Down => {
                            app.pane_down(1);
                            needs_redraw = true;
                        }
                        KeyCode::PageUp => {
                            app.pane_up(10);
                            needs_redraw = true;
                        }
                        KeyCode::PageDown => {
                            app.pane_down(10);
                            needs_redraw = true;
                        }
                        KeyCode::Home if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.pane_top();
                            needs_redraw = true;
                        }
                        KeyCode::End if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.pane_bottom();
                            needs_redraw = true;
                        }
                        KeyCode::Home => {
                            app.pane_top();
                            needs_redraw = true;
                        }
                        KeyCode::End => {
                            app.pane_bottom();
                            needs_redraw = true;
                        }
                        _ => {}
                    },

                    KeyCode::Char(c) => {
                        app.enter_char(c);
                        needs_redraw = true;
                    }
                    KeyCode::Backspace => {
                        app.delete_char();
                        needs_redraw = true;
                    }
                    KeyCode::Enter => {
                        let input = app.take_input();
                        if !input.is_empty() {
                            if input == "/quit" || input == "/exit" {
                                app.quit();
                            } else {
                                on_message(input);
                            }
                        }

                        needs_redraw = true;
                    }
                    KeyCode::Up => {
                        app.history_prev();
                        needs_redraw = true;
                    }
                    KeyCode::Down => {
                        app.history_next();
                        needs_redraw = true;
                    }
                    KeyCode::Left => {
                        app.move_cursor_left();
                        needs_redraw = true;
                    }
                    KeyCode::Right => {
                        app.move_cursor_right();
                        needs_redraw = true;
                    }
                    KeyCode::PageUp => {
                        app.pane_up(10);
                        needs_redraw = true;
                    }
                    KeyCode::PageDown => {
                        app.pane_down(10);
                        needs_redraw = true;
                    }
                    KeyCode::Home if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.pane_top();
                        needs_redraw = true;
                    }
                    KeyCode::End if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.pane_bottom();
                        needs_redraw = true;
                    }
                    KeyCode::Home => {
                        app.cursor_position = 0;
                        needs_redraw = true;
                    }
                    KeyCode::End => {
                        app.cursor_position = app.input.chars().count();
                        needs_redraw = true;
                    }
                    _ => {}
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn copy_selection(app: &TuiApp, clipboard: Option<&mut Clipboard>) -> io::Result<()> {
    let Some(text) = app.selected_text() else {
        return Ok(());
    };

    let Some(clipboard) = clipboard else {
        return Err(io::Error::other("clipboard is unavailable"));
    };

    clipboard.set_text(text).map_err(io::Error::other)
}
