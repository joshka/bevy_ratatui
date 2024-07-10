use bevy::{app::PluginGroupBuilder, prelude::*};

use crate::{bevy_event, error, event, kitty, mouse, terminal};

/// A plugin group that includes all the plugins in the Ratatui crate.
///
/// # Example
///
/// ```rust
/// use bevy::prelude::*;
/// use bevy_ratatui::RatatuiPlugins;
///
/// App::new().add_plugins(RatatuiPlugins::default());
/// ```
pub struct RatatuiPlugins {
    pub enable_kitty_protocol: bool,
    pub enable_mouse_capture: bool,
}

impl Default for RatatuiPlugins {
    fn default() -> Self {
        Self {
            enable_kitty_protocol: true,
            enable_mouse_capture: false,
        }
    }
}

/// A plugin group that includes all the plugins in the Ratatui crate.
///
/// # Example
///
/// ```rust
/// use bevy::prelude::*;
/// use bevy_ratatui::RatatuiPlugins;
///
/// App::new().add_plugins(RatatuiPlugins::default());
/// ```
impl PluginGroup for RatatuiPlugins {
    fn build(self) -> PluginGroupBuilder {
        let mut builder = PluginGroupBuilder::start::<Self>()
            .add(error::ErrorPlugin)
            .add(terminal::TerminalPlugin)
            .add(event::EventPlugin)
            .add(bevy_event::BevyEventPlugin);
        if self.enable_kitty_protocol {
            builder = builder.add(kitty::KittyPlugin);
        }
        if self.enable_mouse_capture {
            builder = builder.add(mouse::MousePlugin);
        }
        builder
    }
}
