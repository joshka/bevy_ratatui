//! Forwards terminal key events to the bevy input system.
//!
//! With this plugin one can use the standard bevy input system with
//! bevy_ratatui:
//!
//! - [`ButtonInput<Key>`] for logical keys,
//! - [`ButtonInput<KeyCode>`] for physical keys,
//! - and `EventReader<`[`KeyboardInput`]`>` for its lowest-level events.
//!
//! The crossterm events are still present and usable with this plugin present.
//!
//! # Usage
//!
//! To enable it, add the [RatatuiPlugins][crate::RatatuiPlugins] with
//! `enable_input_forwarding` set to true.
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_ratatui::*;
//! App::new().add_plugins(RatatuiPlugins {
//!     enable_input_forwarding: true,
//!     ..default()
//! });
//! ```
//!
//! # Example
//!
//! The
//! [bevy_keys](https://github.com/joshka/bevy_ratatui/tree/main/examples/bevy_keys.rs)
//! is like kitty example: it will show you what keys crossterm has received. In
//! addition `bevy_keys` will show what capabilities have been detected and what
//! emulation is being used. This binary can be instructive in determining what
//! capabilities a terminal is setup to provide. (Some terminals require
//! enabling the kitty protocol and some terminals only support part of the
//! protocol.)
//!
//! # Configuration
//!
//! There are two things one can configure: the release key timer, and the
//! emulation policy. In order to explain those, it helps to have a brief
//! overview of what the terminal does and does not provide.
//!
//! ## Terminal
//!
//! Terminal input events are varied. A standard terminal for instance only
//! provides key press events and modifier keys are not given unless in
//! conjunction with another key. Luckily there have been extensions to make key
//! handling more comprehensive like the [kitty comprehensive keyboard handling
//! protocol](https://sw.kovidgoyal.net/kitty/keyboard-protocol/) that
//! bevy_ratatui can use.
//!
//! In order to provide a semblance of bevy's expected input, this plugin can
//! emulate key releases and modifier keys when necessary.
//!
//! ## Capability Emulation
//!
//! There are two capabilities that may or may not be present on the terminal
//! that this plugin concerns it self with:
//!
//! - key releases,
//! - and modifier keys.
//!
//! By default bevy_ratatui will try to use the kitty protocol. If it's present,
//! this plugin will detect whether there are key releases or modifier keys
//! emitted. These are represented in the [Detected] resource.
//!
//! ### Emulation Policy
//!
//! You can choose what to emulate using the [EmulationPolicy] resource. The
//! default policy of [Automatic][EmulationPolicy::Automatic] will emulate
//! whatever capability has not been detected. The
//! [Manual][EmulationPolicy::Manual] policy emulates whatever you ask it to.
//!
//! ## Key Release Time
//!
//! If the terminal does not support sending key release events, this plugin
//! will emulate them by default. An event stream might look like this:
//!
//! `Press A, Press B`
//!
//! This plugin will emit events:
//!
//! `Press A, Release A, Press B, (timer elapses) Release B`
//!
//! The timer is set to one second by default but it can be overwritten. But you
//! can configure it like this:
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_ratatui::bevy_compat::keyboard::*;
//! # let mut app = App::new();
//! app.insert_resource(ReleaseKey(Timer::from_seconds(0.5, TimerMode::Once)));
//! ```
//!
//! # Terminal Choice
//!
//! For the best experience, it is recommended to enable the kitty protocol on
//! your terminal. [See
//! here](https://sw.kovidgoyal.net/kitty/keyboard-protocol/) for a list of
//! terminals implementing this protocol.
use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
};

use bevy::{
    input::{keyboard::KeyboardInput, ButtonState},
    prelude::*,
    window::PrimaryWindow,
};
use crossterm::event::KeyModifiers;

use crate::event::{InputSet, KeyEvent};

bitflags::bitflags! {
    /// Crudely defines some capabilities of terminal. Useful for representing
    /// both detection ([Detect]) and emulation ([EmulationPolicy]).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct Capability: u8 {
        const KEY_RELEASE = 0b0000_0001;
        const MODIFIER = 0b0000_0010;
        const ALL = Self::KEY_RELEASE.bits() | Self::MODIFIER.bits();
    }
}

/// Keyboard emulation policy
///
/// - The [Automatic][EmulationPolicy::Automatic] policy will emulate key release or modifiers if
///   they have not been detected.
///
/// - The [Manual][EmulationPolicy::Manual] policy defines whether modifiers or key releases will be
///   emulated.
///
/// Note: If key releases are emulated and key releases are provided by the
/// terminal, dupliate events may be sent.
#[derive(Debug, Default, Resource)]
pub enum EmulationPolicy {
    /// Emulate everything that has not been detected.
    #[default]
    Automatic,
    /// Define what will be emulated.
    Manual(Capability),
}

impl EmulationPolicy {
    /// Return what capabilities to emulate.
    pub fn emulate_what(&self, detected: &Detected) -> Capability {
        match self {
            EmulationPolicy::Automatic => !detected.0,
            EmulationPolicy::Manual(k) => *k,
        }
    }
}

#[derive(Debug, Resource, Default)]
pub struct Detected(pub Capability);

/// Pass crossterm key events through to bevy's input system.
///
/// You can use bevy's regular `ButtonInput<KeyCode>` or bevy_ratatui's
/// `EventReader<KeyEvent>`.
pub struct KeyboardPlugin;

impl Plugin for KeyboardPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy::input::InputPlugin>() {
            // We need this plugin to submit our events.
            app.add_plugins(bevy::input::InputPlugin);
        }
        if !app.is_plugin_added::<bevy::time::TimePlugin>() {
            // We need this plugin for the delay timer.
            app.add_plugins(bevy::time::TimePlugin);
        }
        app.init_resource::<ReleaseKey>()
            .init_resource::<Detected>()
            .init_resource::<EmulationPolicy>()
            .add_systems(Startup, setup_window)
            .add_systems(
                PreUpdate,
                send_key_events_with_emulation.in_set(InputSet::EmitBevy),
            );
    }
}

/// A new type so we can implement Default and use with `Local`.
#[derive(Debug, Deref, DerefMut)]
struct Modifiers(KeyModifiers);

impl Default for Modifiers {
    fn default() -> Self {
        Self(KeyModifiers::empty())
    }
}

// I wish KeyboardInput had the Hash derive.
// TODO: Drop this if KeyboardInput gets a Hash impl.
#[derive(Deref, DerefMut, PartialEq, Eq)]
struct KeyInput(KeyboardInput);

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
///
/// During processing we read from `.0` and write to `.1`.
#[derive(Default)]
struct LastPress(HashSet<KeyInput>, HashSet<KeyInput>);

impl LastPress {
    fn swap(&mut self) {
        std::mem::swap(&mut self.0, &mut self.1);
    }
}

/// If the terminal does not support sending key release events, this plugin
/// tries to emulate them. An event stream might look like this:
///
/// `Press A, Press B`
///
/// This plugin will emit events:
///
/// `Press A, Release A, Press B, (timer elapses) Release B`
///
/// The timer is set to one second by default but it can be overwritten.
#[derive(Resource, Debug, Deref, DerefMut)]
pub struct ReleaseKey(pub Timer);
// XXX: This could be structured differently:
// enum ReleaseKey {
//     Never,
//     FrameCount(u8),
//     Duration(Duration),
//     Immediate,
// }

impl Default for ReleaseKey {
    /// Set the release key timer for 1 second by default.
    fn default() -> Self {
        ReleaseKey(Timer::from_seconds(1.0, TimerMode::Once))
    }
}

#[allow(clippy::too_many_arguments)]
fn send_key_events_with_emulation(
    mut keys: EventReader<KeyEvent>,
    window: Query<Entity, With<PrimaryWindow>>,
    mut modifiers: Local<Modifiers>,
    mut last_pressed: Local<LastPress>,
    mut keyboard_input: EventWriter<KeyboardInput>,
    mut timer: ResMut<ReleaseKey>,
    time: Res<Time>,
    mut detected: ResMut<Detected>,
    policy: Res<EmulationPolicy>,
) {
    timer.tick(time.delta());
    if keys.is_empty() && !timer.finished() {
        return;
    }
    let bevy_window = window.single();
    for key_event in keys.read() {
        if matches!(key_event.code, crossterm::event::KeyCode::Modifier(_)) {
            detected.0 |= Capability::MODIFIER;
        }

        if let Some((bevy_event, mods, repeated)) = key_event_to_bevy(key_event, bevy_window) {
            if bevy_event.state == ButtonState::Released {
                detected.0 |= Capability::KEY_RELEASE;
            }
            let emulation = policy.emulate_what(&detected);
            if emulation.contains(Capability::MODIFIER) && mods != **modifiers {
                let delta = mods.symmetric_difference(**modifiers);
                for flag in delta {
                    let state = if mods.contains(flag) {
                        // This flag has been added.
                        ButtonState::Pressed
                    } else {
                        // This flag has been removed.
                        ButtonState::Released
                    };
                    let modifier_event =
                        modifier_to_bevy(crossterm_modifier_to_bevy_key(flag), state, bevy_window);
                    keyboard_input.send(modifier_event);
                }
                **modifiers = mods;
            }

            if repeated {
                // Repeated key events are converted to key release events by
                // `key_event_to_bevy()`. But are queued up to emit a key press
                // on the next frame.
                last_pressed.1.insert(KeyInput(bevy_event.clone()));
            }
            if emulation.contains(Capability::KEY_RELEASE) {
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
        keyboard_input.send(KeyboardInput {
            state: match e.0.state {
                ButtonState::Pressed => ButtonState::Released,
                ButtonState::Released => ButtonState::Pressed,
            },
            ..e.0
        });
    }

    if timer.finished()
        && policy
            .emulate_what(&detected)
            .contains(Capability::MODIFIER)
    {
        // Release the modifiers too if we've timed out.
        for flag in **modifiers {
            let state = ButtonState::Released;
            let modifier_event =
                modifier_to_bevy(crossterm_modifier_to_bevy_key(flag), state, bevy_window);
            keyboard_input.send(modifier_event);
        }
        **modifiers = KeyModifiers::empty();
    }
    last_pressed.swap();
    timer.reset();
}

/// Send bevy events without any emulation. This merely shows how simple life is
/// when emulation is not involved. However, it doesn't handle repeated keys
/// though.
#[allow(dead_code)]
fn send_key_events_no_emulation(
    mut keys: EventReader<KeyEvent>,
    window: Query<Entity, With<PrimaryWindow>>,
    mut keyboard_input: EventWriter<KeyboardInput>,
) {
    let bevy_window = window.single();
    for key_event in keys.read() {
        if let Some((bevy_event, _modifiers, _repeated)) = key_event_to_bevy(key_event, bevy_window)
        {
            keyboard_input.send(bevy_event);
        }
    }
}

/// This is a dummy window to satisfy the [KeyboardInput] struct.
fn setup_window(mut commands: Commands) {
    // Insert our window entity so that other parts of our app can use them.
    let _bevy_window = commands.spawn(PrimaryWindow).id();
}

fn modifier_to_bevy(
    modifier: bevy::input::keyboard::Key,
    state: bevy::input::ButtonState,
    window: Entity,
) -> bevy::input::keyboard::KeyboardInput {
    use bevy::input::keyboard::{Key as k, KeyCode as c};
    let key_code = match modifier {
        k::Control => c::ControlLeft,
        k::Shift => c::ShiftLeft,
        k::Alt => c::AltLeft,
        k::Hyper => c::Hyper,
        k::Meta => c::Meta,
        k::Super => c::SuperLeft,
        x => panic!("No such modifier {x:?}"),
    };
    let logical_key = modifier;
    bevy::input::keyboard::KeyboardInput {
        key_code,
        state,
        window,
        logical_key,
    }
}

fn key_event_to_bevy(
    key_event: &crossterm::event::KeyEvent,
    window: Entity,
) -> Option<(
    bevy::input::keyboard::KeyboardInput,
    crossterm::event::KeyModifiers,
    bool,
)> {
    let crossterm::event::KeyEvent {
        code,
        modifiers,
        kind,
        state: _state,
    } = key_event;
    let mut repeated = false;
    let state = match kind {
        crossterm::event::KeyEventKind::Press => bevy::input::ButtonState::Pressed,
        crossterm::event::KeyEventKind::Repeat => {
            repeated = true;
            bevy::input::ButtonState::Released
        }
        crossterm::event::KeyEventKind::Release => bevy::input::ButtonState::Released,
    };
    let key_code = to_bevy_keycode(code);
    let logical_key = to_bevy_key(code);
    key_code
        .zip(logical_key)
        .map(|((key_code, mods), logical_key)| {
            (
                bevy::input::keyboard::KeyboardInput {
                    key_code,
                    state,
                    window,
                    logical_key,
                },
                *modifiers | mods,
                repeated,
            )
        })
}

fn to_bevy_keycode(
    key_code: &crossterm::event::KeyCode,
) -> Option<(
    bevy::input::keyboard::KeyCode,
    crossterm::event::KeyModifiers,
)> {
    use bevy::input::keyboard::KeyCode as b;
    use crossterm::event::{KeyCode as c, KeyModifiers as m};
    let mut mods = crossterm::event::KeyModifiers::empty();
    match key_code {
        c::Backspace => Some(b::Backspace),
        c::Enter => Some(b::Enter),
        c::Left => Some(b::ArrowLeft),
        c::Right => Some(b::ArrowRight),
        c::Up => Some(b::ArrowUp),
        c::Down => Some(b::ArrowDown),
        c::Home => Some(b::Home),
        c::End => Some(b::End),
        c::PageUp => Some(b::PageUp),
        c::PageDown => Some(b::PageDown),
        c::Tab => Some(b::Tab),
        c::BackTab => None,
        c::Delete => Some(b::Delete),
        c::Insert => Some(b::Insert),
        c::F(f) => match f {
            1 => Some(b::F1),
            2 => Some(b::F2),
            3 => Some(b::F3),
            4 => Some(b::F4),
            5 => Some(b::F5),
            6 => Some(b::F6),
            7 => Some(b::F7),
            8 => Some(b::F8),
            9 => Some(b::F9),
            10 => Some(b::F10),
            11 => Some(b::F11),
            12 => Some(b::F12),
            13 => Some(b::F13),
            14 => Some(b::F14),
            15 => Some(b::F15),
            16 => Some(b::F16),
            17 => Some(b::F17),
            18 => Some(b::F18),
            19 => Some(b::F19),
            20 => Some(b::F20),
            31 => Some(b::F31),
            32 => Some(b::F32),
            33 => Some(b::F33),
            34 => Some(b::F34),
            35 => Some(b::F35),
            _ => None,
        },
        c::Char(c) => match c {
            '!' => {
                mods |= m::SHIFT;
                Some(b::Digit1)
            }
            '@' => {
                mods |= m::SHIFT;
                Some(b::Digit2)
            }
            '#' => {
                mods |= m::SHIFT;
                Some(b::Digit3)
            }
            '$' => {
                mods |= m::SHIFT;
                Some(b::Digit4)
            }
            '%' => {
                mods |= m::SHIFT;
                Some(b::Digit5)
            }
            '^' => {
                mods |= m::SHIFT;
                Some(b::Digit6)
            }
            '&' => {
                mods |= m::SHIFT;
                Some(b::Digit7)
            }
            '*' => {
                mods |= m::SHIFT;
                Some(b::Digit8)
            }
            '(' => {
                mods |= m::SHIFT;
                Some(b::Digit9)
            }
            ')' => {
                mods |= m::SHIFT;
                Some(b::Digit0)
            }
            '-' => {
                mods |= m::SHIFT;
                Some(b::Minus)
            }
            '[' => Some(b::BracketLeft),
            ']' => Some(b::BracketRight),
            '{' => {
                mods |= m::SHIFT;
                Some(b::BracketLeft)
            }
            '}' => {
                mods |= m::SHIFT;
                Some(b::BracketRight)
            }
            ',' => Some(b::Comma),
            '=' => Some(b::Equal),
            '<' => {
                mods |= m::SHIFT;
                Some(b::Comma)
            }
            '+' => {
                mods |= m::SHIFT;
                Some(b::Equal)
            }
            '.' => Some(b::Period),
            '>' => {
                mods |= m::SHIFT;
                Some(b::Period)
            }
            '\'' => Some(b::Quote),
            '"' => {
                mods |= m::SHIFT;
                Some(b::Quote)
            }
            ';' => Some(b::Semicolon),
            ':' => {
                mods |= m::SHIFT;
                Some(b::Semicolon)
            }
            '/' => Some(b::Slash),
            '?' => {
                mods |= m::SHIFT;
                Some(b::Slash)
            }
            ' ' => Some(b::Space),
            '1' => Some(b::Digit1),
            '2' => Some(b::Digit2),
            '3' => Some(b::Digit3),
            '4' => Some(b::Digit4),
            '5' => Some(b::Digit5),
            '6' => Some(b::Digit6),
            '7' => Some(b::Digit7),
            '8' => Some(b::Digit8),
            '9' => Some(b::Digit9),
            '0' => Some(b::Digit0),
            'a' => Some(b::KeyA),
            'b' => Some(b::KeyB),
            'c' => Some(b::KeyC),
            'd' => Some(b::KeyD),
            'e' => Some(b::KeyE),
            'f' => Some(b::KeyF),
            'g' => Some(b::KeyG),
            'h' => Some(b::KeyH),
            'i' => Some(b::KeyI),
            'j' => Some(b::KeyJ),
            'k' => Some(b::KeyK),
            'l' => Some(b::KeyL),
            'm' => Some(b::KeyM),
            'n' => Some(b::KeyN),
            'o' => Some(b::KeyO),
            'p' => Some(b::KeyP),
            'q' => Some(b::KeyQ),
            'r' => Some(b::KeyR),
            's' => Some(b::KeyS),
            't' => Some(b::KeyT),
            'u' => Some(b::KeyU),
            'v' => Some(b::KeyV),
            'w' => Some(b::KeyW),
            'x' => Some(b::KeyX),
            'y' => Some(b::KeyY),
            'z' => Some(b::KeyZ),
            'A' => {
                mods |= m::SHIFT;
                Some(b::KeyA)
            }
            'B' => {
                mods |= m::SHIFT;
                Some(b::KeyB)
            }
            'C' => {
                mods |= m::SHIFT;
                Some(b::KeyC)
            }
            'D' => {
                mods |= m::SHIFT;
                Some(b::KeyD)
            }
            'E' => {
                mods |= m::SHIFT;
                Some(b::KeyE)
            }
            'F' => {
                mods |= m::SHIFT;
                Some(b::KeyF)
            }
            'G' => {
                mods |= m::SHIFT;
                Some(b::KeyG)
            }
            'H' => {
                mods |= m::SHIFT;
                Some(b::KeyH)
            }
            'I' => {
                mods |= m::SHIFT;
                Some(b::KeyI)
            }
            'J' => {
                mods |= m::SHIFT;
                Some(b::KeyJ)
            }
            'K' => {
                mods |= m::SHIFT;
                Some(b::KeyK)
            }
            'L' => {
                mods |= m::SHIFT;
                Some(b::KeyL)
            }
            'M' => {
                mods |= m::SHIFT;
                Some(b::KeyM)
            }
            'N' => {
                mods |= m::SHIFT;
                Some(b::KeyN)
            }
            'O' => {
                mods |= m::SHIFT;
                Some(b::KeyO)
            }
            'P' => {
                mods |= m::SHIFT;
                Some(b::KeyP)
            }
            'Q' => {
                mods |= m::SHIFT;
                Some(b::KeyQ)
            }
            'R' => {
                mods |= m::SHIFT;
                Some(b::KeyR)
            }
            'S' => {
                mods |= m::SHIFT;
                Some(b::KeyS)
            }
            'T' => {
                mods |= m::SHIFT;
                Some(b::KeyT)
            }
            'U' => {
                mods |= m::SHIFT;
                Some(b::KeyU)
            }
            'V' => {
                mods |= m::SHIFT;
                Some(b::KeyV)
            }
            'W' => {
                mods |= m::SHIFT;
                Some(b::KeyW)
            }
            'X' => {
                mods |= m::SHIFT;
                Some(b::KeyX)
            }
            'Y' => {
                mods |= m::SHIFT;
                Some(b::KeyY)
            }
            'Z' => {
                mods |= m::SHIFT;
                Some(b::KeyZ)
            }
            _ => None,
        },
        c::Null => None,
        c::Esc => Some(b::Escape),
        c::CapsLock => Some(b::CapsLock),
        c::ScrollLock => Some(b::ScrollLock),
        c::NumLock => Some(b::NumLock),
        c::PrintScreen => Some(b::PrintScreen),
        c::Pause => Some(b::Pause),
        c::Menu => Some(b::ContextMenu),
        c::KeypadBegin => None,
        c::Media(media) => {
            use crossterm::event::MediaKeyCode::*;
            match media {
                Play => Some(b::MediaPlayPause),
                Pause => Some(b::Pause),
                PlayPause => Some(b::MediaPlayPause),
                Reverse => None,
                Stop => Some(b::MediaStop),
                FastForward => Some(b::MediaTrackNext),
                Rewind => Some(b::MediaTrackPrevious),
                TrackNext => Some(b::MediaTrackNext),
                TrackPrevious => Some(b::MediaTrackPrevious),
                Record => None,
                LowerVolume => Some(b::AudioVolumeDown),
                RaiseVolume => Some(b::AudioVolumeUp),
                MuteVolume => Some(b::AudioVolumeMute),
            }
        }
        c::Modifier(modifier) => {
            use crossterm::event::ModifierKeyCode::*;
            match modifier {
                LeftShift => Some(b::ShiftLeft),
                LeftControl => Some(b::ControlLeft),
                LeftAlt => Some(b::AltLeft),
                LeftSuper => Some(b::SuperLeft),
                LeftHyper => Some(b::Hyper),
                LeftMeta => Some(b::Meta),
                RightShift => Some(b::ShiftRight),
                RightControl => Some(b::ControlRight),
                RightAlt => Some(b::AltRight),
                RightSuper => Some(b::SuperRight),
                RightHyper => Some(b::Hyper),
                RightMeta => Some(b::Meta),
                IsoLevel3Shift => None,
                IsoLevel5Shift => None,
            }
        }
    }
    .map(|key_code| (key_code, mods))
}

fn to_bevy_key(key_code: &crossterm::event::KeyCode) -> Option<bevy::input::keyboard::Key> {
    use bevy::input::keyboard::Key as b;
    use crossterm::event::KeyCode as c;
    match key_code {
        c::Backspace => Some(b::Backspace),
        c::Enter => Some(b::Enter),
        c::Left => Some(b::ArrowLeft),
        c::Right => Some(b::ArrowRight),
        c::Up => Some(b::ArrowUp),
        c::Down => Some(b::ArrowDown),
        c::Home => Some(b::Home),
        c::End => Some(b::End),
        c::PageUp => Some(b::PageUp),
        c::PageDown => Some(b::PageDown),
        c::Tab => Some(b::Tab),
        c::BackTab => None,
        c::Delete => Some(b::Delete),
        c::Insert => Some(b::Insert),
        c::F(f) => match f {
            1 => Some(b::F1),
            2 => Some(b::F2),
            3 => Some(b::F3),
            4 => Some(b::F4),
            5 => Some(b::F5),
            6 => Some(b::F6),
            7 => Some(b::F7),
            8 => Some(b::F8),
            9 => Some(b::F9),
            10 => Some(b::F10),
            11 => Some(b::F11),
            12 => Some(b::F12),
            13 => Some(b::F13),
            14 => Some(b::F14),
            15 => Some(b::F15),
            16 => Some(b::F16),
            17 => Some(b::F17),
            18 => Some(b::F18),
            19 => Some(b::F19),
            20 => Some(b::F20),
            31 => Some(b::F31),
            32 => Some(b::F32),
            33 => Some(b::F33),
            34 => Some(b::F34),
            35 => Some(b::F35),
            _ => None,
        },
        c::Char(c) => Some({
            let mut tmp = [0u8; 4];
            let s = c.encode_utf8(&mut tmp);
            b::Character(smol_str::SmolStr::from(s))
        }),
        c::Null => None,
        c::Esc => Some(b::Escape),
        c::CapsLock => Some(b::CapsLock),
        c::ScrollLock => Some(b::ScrollLock),
        c::NumLock => Some(b::NumLock),
        c::PrintScreen => Some(b::PrintScreen),
        c::Pause => Some(b::Pause),
        c::Menu => Some(b::ContextMenu),
        c::KeypadBegin => None,
        c::Media(media) => {
            use crossterm::event::MediaKeyCode::*;
            match media {
                Play => Some(b::MediaPlay),
                Pause => Some(b::Pause),
                PlayPause => Some(b::MediaPlayPause),
                Reverse => None,
                Stop => Some(b::MediaStop),
                FastForward => Some(b::MediaFastForward),
                Rewind => Some(b::MediaRewind),
                TrackNext => Some(b::MediaTrackNext),
                TrackPrevious => Some(b::MediaTrackPrevious),
                Record => Some(b::MediaRecord),
                LowerVolume => Some(b::AudioVolumeDown),
                RaiseVolume => Some(b::AudioVolumeUp),
                MuteVolume => Some(b::AudioVolumeMute),
            }
        }
        c::Modifier(modifier) => {
            use crossterm::event::ModifierKeyCode::*;
            match modifier {
                LeftShift => Some(b::Shift),
                LeftControl => Some(b::Control),
                LeftAlt => Some(b::Alt),
                LeftSuper => Some(b::Super),
                LeftHyper => Some(b::Hyper),
                LeftMeta => Some(b::Meta),
                RightShift => Some(b::Shift),
                RightControl => Some(b::Control),
                RightAlt => Some(b::Alt),
                RightSuper => Some(b::Super),
                RightHyper => Some(b::Hyper),
                RightMeta => Some(b::Meta),
                IsoLevel3Shift => Some(b::AltGraph),
                IsoLevel5Shift => None,
            }
        }
    }
}

fn crossterm_modifier_to_bevy_key(
    modifier: crossterm::event::KeyModifiers,
) -> bevy::input::keyboard::Key {
    let mut i = modifier.into_iter();
    let modifier = i.next().expect("mod");
    use bevy::input::keyboard::Key as k;
    use crossterm::event::KeyModifiers as c;
    let result = match modifier {
        c::SHIFT => k::Shift,
        c::CONTROL => k::Control,
        c::ALT => k::Alt,
        c::SUPER => k::Super,
        c::HYPER => k::Hyper,
        c::META => k::Meta,
        x => panic!("Given a modifier of {x:?}"),
    };
    assert!(i.next().is_none());
    result
}
