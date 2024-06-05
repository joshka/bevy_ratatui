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
    cursor,
    event::{
        DisableMouseCapture, EnableMouseCapture, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    terminal::{
        disable_raw_mode, enable_raw_mode, supports_keyboard_enhancement, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
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
        stdout()
            .execute(EnterAlternateScreen)?
            .execute(EnableMouseCapture)?;
        enable_raw_mode()?;
        let backend = CrosstermBackend::new(stdout());
        let terminal = ratatui::Terminal::new(backend)?;
        Ok(RatatuiContext(terminal))
    }

    /// Enables support for the kitty keyboard protocol.
    /// https://sw.kovidgoyal.net/kitty/keyboard-protocol/
    ///
    /// Provides additional information involving keyboard events. For example, key release events
    /// will be reported.
    ///
    /// Refer to the above link for a list of terminals that support the
    /// protocol. An `Ok` result is not a guarantee that all features are supported: you should
    /// have fallbacks that you use until you detect the event type you are looking for.
    pub fn enable_kitty_protocol(&mut self) -> io::Result<()> {
        if matches!(supports_keyboard_enhancement(), Ok(true)) {
            stdout().execute(PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::all()))?;
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Kitty keyboard protocol is not supported by this terminal.",
            ))
        }
    }

    /// Restores the terminal, leaving the alternate screen and disabling raw mode.
    pub fn restore() -> io::Result<()> {
        stdout()
            .execute(PopKeyboardEnhancementFlags)?
            .execute(LeaveAlternateScreen)?
            .execute(DisableMouseCapture)?
            .execute(cursor::Show)?;
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
