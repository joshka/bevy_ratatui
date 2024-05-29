use bevy::{app::AppExit, prelude::*};
use color_eyre::{
    config::{EyreHook, HookBuilder, PanicHook},
    eyre,
};
use crossterm::{
    event::{Event::Key, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::backend::CrosstermBackend;
use std::{
    io::{self, stdout, Stdout},
    panic,
};

pub struct RatatuiPlugin;

impl Plugin for RatatuiPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<CrosstermKeyEvent>()
            .add_systems(Startup, setup_terminal.pipe(exit_on_error))
            .add_systems(PreUpdate, crossterm_event_system.pipe(exit_on_error));
    }
}

#[derive(Debug, Deref, Event)]
pub struct CrosstermKeyEvent(pub KeyEvent);

fn setup_terminal(mut commands: Commands) -> color_eyre::Result<()> {
    setup_error_handling()?;
    let terminal = RatatuiContext::init()?;
    commands.insert_resource(terminal);
    Ok(())
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

fn crossterm_event_system(
    mut app_exit: EventWriter<AppExit>,
    mut key_events: EventWriter<CrosstermKeyEvent>,
) -> color_eyre::Result<()> {
    while crossterm::event::poll(std::time::Duration::ZERO)? {
        match crossterm::event::read()? {
            Key(event) if event.kind == KeyEventKind::Press => {
                match (event.modifiers, event.code) {
                    (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                        app_exit.send_default();
                    }
                    _ => {}
                }
                key_events.send(CrosstermKeyEvent(event));
            }
            _ => {}
        }
    }
    Ok(())
}

/// A wrapper around ratatui::Terminal that automatically enters and leaves the alternate screen.
#[derive(Resource, Deref, DerefMut)]
pub struct RatatuiContext(ratatui::Terminal<CrosstermBackend<Stdout>>);

impl RatatuiContext {
    pub fn init() -> io::Result<Self> {
        stdout().execute(EnterAlternateScreen)?;
        enable_raw_mode()?;
        let backend = CrosstermBackend::new(stdout());
        let terminal = ratatui::Terminal::new(backend)?;
        Ok(RatatuiContext(terminal))
    }

    pub fn restore() -> io::Result<()> {
        stdout().execute(LeaveAlternateScreen)?;
        disable_raw_mode()?;
        Ok(())
    }
}

impl Drop for RatatuiContext {
    fn drop(&mut self) {
        RatatuiContext::restore().unwrap();
    }
}
