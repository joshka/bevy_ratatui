//! A collection of plugins for building terminal-based applications with [Bevy] and [Ratatui].
//!
//! # Example
//!
//! ```rust,no_run
//! use bevy::{
//!     app::{AppExit, ScheduleRunnerPlugin},
//!     prelude::*,
//! };
//! use bevy_ratatui::{
//!     error::exit_on_error, event::KeyEvent, terminal::RatatuiContext, RatatuiPlugins,
//! };
//!
//! fn main() {
//!     let wait_duration = std::time::Duration::from_secs_f64(1. / 60.); // 60 FPS
//!     App::new()
//!         .add_plugins(RatatuiPlugins)
//!         .add_plugins(ScheduleRunnerPlugin::run_loop(wait_duration))
//!         .add_systems(PreUpdate, keyboard_input_system)
//!         .add_systems(Update, hello_world.pipe(exit_on_error))
//!         .run();
//! }
//!
//! fn hello_world(mut context: ResMut<RatatuiContext>) -> color_eyre::Result<()> {
//!     context.draw(|frame| {
//!         let text = ratatui::text::Text::raw("hello world\nPress 'q' to Quit");
//!         frame.render_widget(text, frame.size())
//!     })?;
//!     Ok(())
//! }
//!
//! fn keyboard_input_system(mut events: EventReader<KeyEvent>, mut exit: EventWriter<AppExit>) {
//!     use crossterm::event::KeyCode; // beware bevy prelude also has a KeyCode enum
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
//!
//! See the [examples] directory for more examples.
//!
//! [Bevy]: https://bevyengine.org
//! [Ratatui]: https://ratatui.rs
//! [examples]: https://github.com/joshka/bevy_ratatui/tree/main/examples
use bevy::{app::PluginGroupBuilder, prelude::*};

pub mod error;
pub mod event;
pub mod terminal;

/// A plugin group that includes all the plugins in the Ratatui crate.
///
/// # Example
///
/// ```rust
/// use bevy::prelude::*;
/// use bevy_ratatui::RatatuiPlugins;
///
/// App::new().add_plugins(RatatuiPlugins);
/// ```
pub struct RatatuiPlugins;

impl PluginGroup for RatatuiPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(error::ErrorPlugin)
            .add(terminal::TerminalPlugin)
            .add(event::EventPlugin)
    }
}
