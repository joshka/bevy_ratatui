//! Error handling for the app.
//!
//! This module provides a plugin that sets up error handling for the app. It installs hooks for
//! panic and error handling that restore the terminal before printing the panic or error message.
//! This ensures that the error message is not messed up by the terminal state.
//!
//! The `exit_on_error` function is used to exit the app if an error occurs. It is used to pipe
//! results from functions that return `Result` to the `exit_on_error` system. If the result is an
//! error, the error is logged and the app is exited.
use std::panic;

use bevy::{app::AppExit, prelude::*};
use color_eyre::{
    self,
    config::{EyreHook, HookBuilder, PanicHook},
    eyre, Result,
};

use crate::terminal::RatatuiContext;

/// A plugin that sets up error handling.
///
/// This plugin installs hooks for panic and error handling that restore the terminal before
/// printing the panic or error message. This ensures that the error message is not messed up by the
/// terminal state.
pub struct ErrorPlugin;

/// A system that sets up error handling.
///
/// This system sets up hooks for panic and error handling. It is used to ensure that the terminal
/// is restored before printing the panic or error message.
impl Plugin for ErrorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup.pipe(exit_on_error));
    }
}

/// Installs hooks for panic and error handling.
///
/// Makes the app resilient to panics and errors by restoring the terminal before printing the
/// panic or error message. This prevents error messages from being messed up by the terminal
/// state.
pub fn setup() -> Result<()> {
    let (panic_hook, eyre_hook) = HookBuilder::default().into_hooks();
    set_panic_hook(panic_hook);
    set_error_hook(eyre_hook)?;
    Ok(())
}

/// Install a panic hook that restores the terminal before printing the panic.
fn set_panic_hook(panic_hook: PanicHook) {
    let panic_hook = panic_hook.into_panic_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = RatatuiContext::restore();
        panic_hook(panic_info);
    }));
}

/// Install an error hook that restores the terminal before printing the error.
fn set_error_hook(eyre_hook: EyreHook) -> Result<()> {
    let eyre_hook = eyre_hook.into_eyre_hook();
    eyre::set_hook(Box::new(move |error| {
        let _ = RatatuiContext::restore();
        eyre_hook(error)
    }))?;
    Ok(())
}

/// Exits the app if an error occurs.
///
/// This is used to pipe results from functions that return `Result` to the `exit_on_error` system.
/// If the result is an error, the error is logged and the app is exited.
pub fn exit_on_error(In(result): In<Result<()>>, mut app_exit: EventWriter<AppExit>) {
    if let Err(err) = result {
        error!("Error: {:?}", err);
        app_exit.send_default();
    }
}
