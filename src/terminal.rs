use bevy::prelude::*;
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::backend::CrosstermBackend;
use std::io::{self, stdout, Stdout};

pub fn setup_terminal(mut commands: Commands) -> color_eyre::Result<()> {
    let terminal = RatatuiContext::init()?;
    commands.insert_resource(terminal);
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
