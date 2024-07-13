//! Enhanced kitty keyboard protocol.
use std::io::{self, stdout};

use bevy::prelude::*;
use crossterm::{
    event::{KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    terminal::supports_keyboard_enhancement,
    ExecutableCommand,
};

use crate::terminal;

pub struct KittyPlugin;

impl Plugin for KittyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup.after(terminal::setup));
    }
}

fn setup(mut commands: Commands) {
    if enable_kitty_protocol().is_ok() {
        commands.insert_resource(KittyEnabled);
    }
}

#[derive(Resource)]
pub struct KittyEnabled;

impl Drop for KittyEnabled {
    fn drop(&mut self) {
        let _ = disable_kitty_protocol();
    }
}

/// Enables support for the [kitty keyboard protocol]
///
/// Provides additional information involving keyboard events. For example, key release events will
/// be reported.
///
/// Refer to the above link for a list of terminals that support the protocol. An `Ok` result is not
/// a guarantee that all features are supported: you should have fallbacks that you use until you
/// detect the event type you are looking for.
///
/// [kitty keyboard protocol]: https://sw.kovidgoyal.net/kitty/keyboard-protocol/
pub fn enable_kitty_protocol() -> io::Result<()> {
    if supports_keyboard_enhancement()? {
        stdout().execute(PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::all()))?;
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Kitty keyboard protocol is not supported by this terminal.",
    ))
}

/// Disables the [kitty keyboard protocol]
///
/// [kitty keyboard protocol]: https://sw.kovidgoyal.net/kitty/keyboard-protocol/
pub fn disable_kitty_protocol() -> io::Result<()> {
    stdout().execute(PopKeyboardEnhancementFlags)?;
    Ok(())
}
