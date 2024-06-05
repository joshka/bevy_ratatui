use std::io::stdout;

use bevy::prelude::*;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    ExecutableCommand,
};

use crate::error::exit_on_error;

pub struct MousePlugin;

impl Plugin for MousePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup.pipe(exit_on_error));
    }
}

#[derive(Resource, Default)]
pub struct MouseCaptureEnabled;

fn setup(mut commands: Commands) -> color_eyre::Result<()> {
    stdout().execute(EnableMouseCapture)?;
    commands.insert_resource(MouseCaptureEnabled);
    Ok(())
}

impl Drop for MouseCaptureEnabled {
    fn drop(&mut self) {
        let _ = stdout().execute(DisableMouseCapture);
    }
}
