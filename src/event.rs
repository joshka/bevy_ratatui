//! Event handling.
//!
//! This module provides a plugin for handling events, and a wrapper around
//! `crossterm::event::KeyEvent`.
//!
//! # Example
//!
//! ```rust
//! use bevy::{app::AppExit, prelude::*};
//! use bevy_ratatui::event::KeyEvent;
//! use crossterm::event::KeyCode;
//!
//! fn keyboard_input_system(mut events: EventReader<KeyEvent>, mut exit: EventWriter<AppExit>) {
//!     for event in events.read() {
//!         match event.code {
//!             KeyCode::Char('q') | KeyCode::Esc => {
//!                 exit.send(AppExit);
//!             }
//!             _ => {}
//!         }
//!     }
//! }
//! ```
use std::time::Duration;

use bevy::{app::AppExit, prelude::*};
use color_eyre::Result;
use crossterm::event::{self, KeyCode, KeyEventKind, KeyModifiers};

use crate::error::exit_on_error;

/// A plugin for handling events.
///
/// This plugin adds the `KeyEvent` event, and a system that reads events from crossterm and sends
/// them to the `KeyEvent` event.
pub struct EventPlugin;

impl Plugin for EventPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<KeyEvent>()
            .add_systems(PreUpdate, crossterm_event_system.pipe(exit_on_error));
    }
}

/// Wrapper around `crossterm::event::KeyEvent`.
#[derive(Debug, Deref, Event, PartialEq, Eq, Clone, Hash)]
pub struct KeyEvent(pub event::KeyEvent);

/// System that reads events from crossterm and sends them to the `KeyEvent` event.
///
/// This system reads events from crossterm and sends them to the `KeyEvent` event. It also sends
/// an `AppExit` event when `Ctrl+C` is pressed.
pub fn crossterm_event_system(
    mut exit: EventWriter<AppExit>,
    mut keys: EventWriter<KeyEvent>,
) -> Result<()> {
    while event::poll(Duration::ZERO)? {
        match event::read()? {
            event::Event::Key(event) if event.kind == KeyEventKind::Press => {
                if event.modifiers == KeyModifiers::CONTROL && event.code == KeyCode::Char('c') {
                    exit.send_default();
                }
                keys.send(KeyEvent(event));
            }
            _ => {}
        }
    }
    Ok(())
}
