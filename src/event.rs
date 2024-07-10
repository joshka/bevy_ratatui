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
//!                 exit.send_default();
//!             }
//!             _ => {}
//!         }
//!     }
//! }
//! ```
use std::time::Duration;

use bevy::{app::AppExit, prelude::*};
use color_eyre::Result;
use crossterm::event::{self, Event::Key, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::Size;

use crate::error::exit_on_error;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum InputSet {
    Pre,
    CrosstermEmit,
    BevyEmit,
    Post,
}

/// A plugin for handling events.
///
/// This plugin adds the `KeyEvent` event, and a system that reads events from crossterm and sends
/// them to the `KeyEvent` event.
pub struct EventPlugin;

impl Plugin for EventPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<KeyEvent>()
            .add_event::<MouseEvent>()
            .add_event::<FocusEvent>()
            .add_event::<ResizeEvent>()
            .add_event::<PasteEvent>()
            .add_event::<CrosstermEvent>()
            .configure_sets(
                Update,
                (InputSet::Pre, InputSet::CrosstermEmit, InputSet::BevyEmit, InputSet::Post).chain(),
            )
            .add_systems(
                PreUpdate,
                crossterm_event_system
                    .pipe(exit_on_error)
                    .in_set(InputSet::CrosstermEmit),
            );
    }
}

/// An event that is sent whenever an event is read from crossterm.
#[derive(Debug, Deref, Event, PartialEq, Eq, Clone, Hash)]
pub struct CrosstermEvent(pub event::Event);

/// An event that is sent whenever a key event is read from crossterm.
#[derive(Debug, Deref, Event, PartialEq, Eq, Clone, Hash)]
pub struct KeyEvent(pub event::KeyEvent);

/// An event that is sent whenever a mouse event is read from crossterm.
#[derive(Debug, Clone, Copy, Event, PartialEq, Eq, Deref)]
pub struct MouseEvent(pub event::MouseEvent);

/// An event that is sent when the terminal gains or loses focus.
#[derive(Debug, Clone, Copy, Event, PartialEq, Eq)]
pub enum FocusEvent {
    Gained,
    Lost,
}

/// An event that is sent when the terminal is resized.
#[derive(Debug, Clone, Copy, Event, PartialEq, Eq, Deref)]
pub struct ResizeEvent(pub Size);

/// An event that is sent when text is pasted into the terminal.
#[derive(Debug, Clone, Event, PartialEq, Eq, Deref)]
pub struct PasteEvent(pub String);

/// System that reads events from crossterm and sends them to the `KeyEvent` event.
///
/// This system reads events from crossterm and sends them to the `KeyEvent` event. It also sends
/// an `AppExit` event when `Ctrl+C` is pressed.
pub fn crossterm_event_system(
    mut events: EventWriter<CrosstermEvent>,
    mut keys: EventWriter<KeyEvent>,
    mut mouse: EventWriter<MouseEvent>,
    mut focus: EventWriter<FocusEvent>,
    mut paste: EventWriter<PasteEvent>,
    mut resize: EventWriter<ResizeEvent>,
    mut exit: EventWriter<AppExit>,
) -> Result<()> {
    while event::poll(Duration::ZERO)? {
        let event = event::read()?;
        match event {
            Key(event) => {
                if event.kind == KeyEventKind::Press
                    && event.modifiers == KeyModifiers::CONTROL
                    && event.code == KeyCode::Char('c')
                {
                    exit.send_default();
                }

                keys.send(KeyEvent(event));
            }
            event::Event::FocusLost => {
                focus.send(FocusEvent::Lost);
            }
            event::Event::FocusGained => {
                focus.send(FocusEvent::Gained);
            }
            event::Event::Mouse(event) => {
                mouse.send(MouseEvent(event));
            }
            event::Event::Paste(ref s) => {
                paste.send(PasteEvent(s.clone()));
            }
            event::Event::Resize(columns, rows) => {
                resize.send(ResizeEvent(Size::new(columns, rows)));
            }
        }
        events.send(CrosstermEvent(event));
    }
    Ok(())
}
