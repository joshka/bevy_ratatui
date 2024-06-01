//! This module contains the terminal plugin and the RatatuiContext resource.
//!
//! [`TerminalPlugin`] initializes the terminal, entering the alternate screen and enabling raw
//! mode. It also restores the terminal when the app is dropped.
//!
//! [`RatatuiContext`] is a wrapper [`Resource`] around ratatui::Terminal that automatically enters
//! and leaves the alternate screen.
use std::io::{self, stdout, Stdout};

use bevy::prelude::*;
use color_eyre::Result;
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::backend::CrosstermBackend;

use crate::error::exit_on_error;

/// A plugin that sets up the terminal.
///
/// This plugin initializes the terminal, entering the alternate screen and enabling raw mode. It
/// also restores the terminal when the app is dropped.
pub struct TerminalPlugin;

impl Plugin for TerminalPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup.pipe(exit_on_error));
    }
}

/// A startup system that sets up the terminal.
pub fn setup(mut commands: Commands) -> Result<()> {
    let terminal = RatatuiContext::init()?;
    commands.insert_resource(terminal);
    Ok(())
}

/// A wrapper around ratatui::Terminal that automatically enters and leaves the alternate screen.
///
/// This resource is used to draw to the terminal. It automatically enters the alternate screen when
/// it is initialized, and leaves the alternate screen when it is dropped.
///
/// # Example
///
/// ```rust
/// use bevy::prelude::*;
/// use bevy_ratatui::terminal::RatatuiContext;
///
/// fn draw_system(mut context: ResMut<RatatuiContext>) {
///     context.draw(|frame| {
///         // Draw widgets etc. to the terminal
///     });
/// }
/// ```
#[derive(Resource, Deref, DerefMut)]
pub struct RatatuiContext(ratatui::Terminal<CrosstermBackend<Stdout>>);

impl RatatuiContext {
    /// Initializes the terminal, entering the alternate screen and enabling raw mode.
    pub fn init() -> io::Result<Self> {
        stdout().execute(EnterAlternateScreen)?;
        enable_raw_mode()?;
        let backend = CrosstermBackend::new(stdout());
        let terminal = ratatui::Terminal::new(backend)?;
        Ok(RatatuiContext(terminal))
    }

    /// Restores the terminal, leaving the alternate screen and disabling raw mode.
    pub fn restore() -> io::Result<()> {
        stdout().execute(LeaveAlternateScreen)?;
        disable_raw_mode()?;
        Ok(())
    }
}

/// Restores the terminal when the app is dropped.
///
/// Any errors that occur when restoring the terminal are logged and ignored.
impl Drop for RatatuiContext {
    fn drop(&mut self) {
        if let Err(err) = RatatuiContext::restore() {
            eprintln!("Failed to restore terminal: {}", err);
        }
    }
}
