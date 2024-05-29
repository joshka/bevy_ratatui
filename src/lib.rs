use bevy::{app::PluginGroupBuilder, prelude::*};

pub mod error;
pub mod event;
pub mod terminal;

pub struct RatatuiPlugins;

impl PluginGroup for RatatuiPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(error::ErrorPlugin)
            .add(terminal::TerminalPlugin)
            .add(event::EventPlugin)
    }
}
