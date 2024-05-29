use std::panic;

use bevy::{app::AppExit, prelude::*};
use color_eyre::{
    self,
    config::{EyreHook, HookBuilder, PanicHook},
    eyre,
};

use crate::terminal::RatatuiContext;

pub struct ErrorPlugin;

impl Plugin for ErrorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_error_handling.pipe(exit_on_error));
    }
}

/// Installs hooks for panic and error handling.
///
/// Makes the app resilient to panics and errors by restoring the terminal before printing the
/// panic or error message. This prevents error messages from being messed up by the terminal
/// state.
pub fn setup_error_handling() -> color_eyre::Result<()> {
    let (panic_hook, eyre_hook) = HookBuilder::default().into_hooks();
    install_panic_hook(panic_hook);
    install_error_hook(eyre_hook)?;
    Ok(())
}

/// Install a panic hook that restores the terminal before printing the panic.
fn install_panic_hook(panic_hook: PanicHook) {
    let panic_hook = panic_hook.into_panic_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = RatatuiContext::restore();
        panic_hook(panic_info);
    }));
}

/// Install an error hook that restores the terminal before printing the error.
fn install_error_hook(eyre_hook: EyreHook) -> color_eyre::Result<()> {
    let eyre_hook = eyre_hook.into_eyre_hook();
    eyre::set_hook(Box::new(move |error| {
        let _ = RatatuiContext::restore();
        eyre_hook(error)
    }))?;
    Ok(())
}

pub fn exit_on_error(In(result): In<color_eyre::Result<()>>, mut app_exit: EventWriter<AppExit>) {
    if let Err(err) = result {
        error!("Error: {:?}", err);
        app_exit.send_default();
    }
}
