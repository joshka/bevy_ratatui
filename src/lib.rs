use bevy::prelude::*;
use error::{exit_on_error, setup_error_handling};
use event::{crossterm_event_system, KeyEvent};
use terminal::setup_terminal;

pub mod error;
pub mod event;
pub mod terminal;

pub struct RatatuiPlugin;

impl Plugin for RatatuiPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<KeyEvent>()
            .add_systems(Startup, setup_error_handling.pipe(exit_on_error))
            .add_systems(Startup, setup_terminal.pipe(exit_on_error))
            .add_systems(PreUpdate, crossterm_event_system.pipe(exit_on_error));
    }
}
