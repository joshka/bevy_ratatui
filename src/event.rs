use bevy::{app::AppExit, prelude::*};
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};

/// Wrapper around `crossterm::event::KeyEvent`.
#[derive(Debug, Deref, Event, PartialEq, Eq, Clone, Hash)]
pub struct KeyEvent(pub crossterm::event::KeyEvent);

pub fn crossterm_event_system(
    mut app_exit: EventWriter<AppExit>,
    mut key_events: EventWriter<KeyEvent>,
) -> color_eyre::Result<()> {
    while crossterm::event::poll(std::time::Duration::ZERO)? {
        match crossterm::event::read()? {
            crossterm::event::Event::Key(event) if event.kind == KeyEventKind::Press => {
                match (event.modifiers, event.code) {
                    (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                        app_exit.send_default();
                    }
                    _ => {}
                }
                key_events.send(KeyEvent(event));
            }
            _ => {}
        }
    }
    Ok(())
}
