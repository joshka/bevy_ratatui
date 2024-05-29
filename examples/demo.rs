use core::panic;
use std::time::Duration;

use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    prelude::*,
};
use bevy_ratatui::{
    error::exit_on_error, event::KeyEvent, terminal::RatatuiContext, RatatuiPlugin,
};
use crossterm::event::KeyCode;

fn main() {
    let frame_rate = Duration::from_secs_f64(1. / 60.);
    App::new()
        .add_plugins((
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(frame_rate)),
            RatatuiPlugin,
        ))
        .add_systems(
            Update,
            (
                ui_system.pipe(exit_on_error),
                keyboard_input_system.pipe(exit_on_error),
            ),
        )
        .run();
}

fn ui_system(mut context: ResMut<RatatuiContext>) -> color_eyre::Result<()> {
    context.draw(|frame| {
        frame.render_widget("Hello World!", frame.size());
    })?;
    Ok(())
}

fn keyboard_input_system(
    mut events: EventReader<KeyEvent>,
    mut app_exit: EventWriter<AppExit>,
) -> color_eyre::Result<()> {
    for event in events.read() {
        match event.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                app_exit.send_default();
            }
            KeyCode::Char('p') => {
                panic!("Panic!");
            }
            _ => {}
        }
    }
    Ok(())
}
