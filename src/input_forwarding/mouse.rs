use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
};

use bevy::{
    input::{mouse::MouseButtonInput, ButtonState},
    prelude::*,
    window::PrimaryWindow,
};
use crossterm::event::KeyModifiers;

use crate::{
    event::{InputSet, KeyEvent},
    kitty::KittyEnabled,
};

/// Pass crossterm key events through to bevy's input system.
///
/// You can use bevy's regular `ButtonInput<KeyCode>` or bevy_ratatui's
/// `EventReader<KeyEvent>`.
pub struct MousePlugin;

impl Plugin for MousePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy::input::InputPlugin)
           .add_systems(PreUpdate, send_mouse_events.in_set(InputSet::EmitBevy));
    }
}

#[derive(Debug, Deref, DerefMut)]
struct Modifiers(KeyModifiers);

impl Default for Modifiers {
    fn default() -> Self {
        Self(KeyModifiers::empty())
    }
}

// I wish MouseButtonInput had the Hash derive.
// TODO: Drop this if MouseButtonInput get a Hash impl.
#[derive(Deref, DerefMut, PartialEq, Eq)]
struct KeyInput(MouseButtonInput);

impl Hash for KeyInput {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.key_code.hash(state);
        self.logical_key.hash(state);
        self.state.hash(state);
        self.window.hash(state);
    }
}

/// This is a double-buffer for the last keys pressed.
///
/// - `.0` is current frame's last keys pressed.
/// - `.1` is the next frame's last keys pressed.
#[derive(Default)]
struct LastPress(HashSet<KeyInput>, HashSet<KeyInput>);

impl LastPress {
    fn swap(&mut self) {
        std::mem::swap(&mut self.0, &mut self.1);
    }
}

fn send_mouse_events(
    mut keys: EventReader<KeyEvent>,
    kitty_enabled: Option<Res<KittyEnabled>>,
    window: Query<Entity, With<PrimaryWindow>>,
    mut modifiers: Local<Modifiers>,
    mut last_pressed: Local<LastPress>,
    mut keyboard_input: EventWriter<MouseButtonInput>,
) {
    let bevy_window = window.single();
    for key_event in keys.read() {
        if let Some((bevy_event, mods)) = key_event_to_bevy(key_event, bevy_window) {
            if mods != **modifiers {
                let delta = mods.symmetric_difference(**modifiers);
                for flag in delta {
                    let state = if mods.contains(flag) {
                        // This flag has been added.
                        bevy::input::ButtonState::Pressed
                    } else {
                        // This flag has been removed.
                        bevy::input::ButtonState::Released
                    };
                    keyboard_input.send(modifier_to_bevy(
                        crossterm_modifier_to_bevy_key(flag),
                        state,
                        bevy_window,
                    ));
                }
                **modifiers = mods;
            }
            if kitty_enabled.is_none() {
                // Must send the release events ourselves.
                let wrapped = KeyInput(bevy_event.clone());
                if let Some(last_press) = last_pressed.0.take(&wrapped) {
                    // It's being held down. Pass to the next frame.
                    last_pressed.1.insert(last_press);
                } else {
                    last_pressed.1.insert(wrapped);
                    keyboard_input.send(bevy_event);
                }
            } else {
                keyboard_input.send(bevy_event);
            }
        }
    }
    for e in last_pressed.0.drain() {
        keyboard_input.send(MouseButtonInput {
            state: ButtonState::Released,
            ..e.0
        });
    }
    last_pressed.swap();
}
