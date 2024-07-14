//! Input forwarding for the keyboard
//!
//! Forwards terminal key events to the bevy input system.
//!
//! With this plugin one can use the standard bevy input system like
//! [`ButtonInput`][bevy::input::ButtonInput] with bevy_ratatui:
//!
//! - `ButtonInput`[`<Key>`][bevy::input::keyboard::Key] for logical keys,
//! - `ButtonInput`[`<KeyCode>`][bevy::input::keyboard::KeyCode] for physical keys,
//! - and `EventReader<`[`KeyboardInput`][bevy::input::keyboard::KeyboardInput]`>` for its
//!   lowest-level events.
//!
//! The crossterm events are still present and usable with this plugin present.
//!
//! # Usage
//!
//! This plugin is automatically included in the [RatatuiPlugins] group. If you are not using the
//! `RatatuiPlugins` group, you can add the plugin to your app like so:
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_ratatui::input::*;
//! # let mut app = App::new();
//! app.add_plugin(KeyboardPlugin);
//! ```
//!
//! # Example
//!
//! The [bevy_keys](https://github.com/joshka/bevy_ratatui/tree/main/examples/bevy_keys.rs) example
//! shows the keys crossterm has received and the bevy keys that are emitted. In addition it will
//! show which capabilities have been detected and the emulation that is being used.
//!
//! This example can be instructive in determining which capabilities a terminal may or may not
//! provide. Some terminals require enabling the kitty protocol and some terminals only support part
//! of the protocol.
//!
//! # Configuration
//!
//! There are two things you can configure on this plugin: the release key timer, and the emulation
//! policy. In order to explain those, it helps to have a brief overview of what the terminal may or
//! may not provide.
//!
//! ## Terminal
//!
//! Terminal input events are varied. A standard terminal for instance only provides key press
//! events and modifier keys are not given except in conjunction with another key. Luckily
//! extensions exist to make key handling more comprehensive like the [kitty comprehensive keyboard
//! handling protocol](https://sw.kovidgoyal.net/kitty/keyboard-protocol/) that bevy_ratatui can
//! use.
//!
//! In order to provide a semblance of expected input for bevy, this plugin can emulate key releases
//! and modifier keys when necessary.
//!
//! ## Capability Emulation
//!
//! There are two capabilities that may or may not be present on the terminal that this plugin
//! concerns it self with:
//!
//! - key releases,
//! - and modifier keys.
//!
//! By default bevy_ratatui will try to use the kitty protocol. If it's present, this plugin will
//! detect whether there are key releases or modifier keys emitted. These are represented in the
//! [Detected] resource.
//!
//! ### Emulation Policy
//!
//! You can choose what to emulate using the [EmulationPolicy] resource. The default policy of
//! [Automatic][EmulationPolicy::Automatic] will emulate whatever capability has not been detected.
//! The [Manual][EmulationPolicy::Manual] policy emulates whatever you ask it to.
//!
//! ## Key Release Time
//!
//! If the terminal does not support sending key release events, this plugin can emulate them. By
//! default it will use a timeout to emit a release key for the last key pressed. An event stream
//! might look like this:
//!
//! `Press A, Press B`
//!
//! This plugin will emit events:
//!
//! `Press A, Release A, Press B, (timer finishes) Release B`
//!
//! The timer is set to one second by default but it can be configured like so:
//!
//! ```no_run
//! # use std::time::Duration;
//! # use bevy::prelude::*;
//! # use bevy_ratatui::input::*;
//! # let mut app = App::new();
//! app.insert_resource(ReleaseKey::Duration(Duration::from_secs_f32(0.5)));
//! ```
//!
//! There are other policies one can choose by configuring [ReleaseKey]. See its documentation for
//! more details.
//!
//! # Terminal Choice
//!
//! For the best experience, it is recommended to enable the kitty protocol on your terminal. [See
//! here](https://sw.kovidgoyal.net/kitty/keyboard-protocol/) for a list of terminals implementing
//! this protocol.
use std::{collections::HashSet, time::Duration};

use bevy::{
    input::{keyboard::KeyboardInput, ButtonState},
    prelude::*,
};
use crossterm::event::KeyModifiers;

use crate::{
    event::{InputSet, KeyEvent},
    terminal::DummyWindow,
};

/// Pass crossterm key events through to the bevy input system. See
/// [input_forwarding][crate::input_forwarding] for more details.
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
        app.init_resource::<KeyReleaseMode>()
            .init_resource::<KeyReleaseTimer>()
            .init_resource::<DetectedCapabilities>()
            .init_resource::<EmulationPolicy>()
            .init_resource::<EmulateCapabilities>()
            .add_systems(
                PreUpdate,
                detect_capabilities.in_set(InputSet::CheckEmulation),
            )
            .add_systems(
                PreUpdate,
                (
                    (emulate_release_keys, emulate_release_modifiers).chain(),
                    emulate_key_events.run_if(resource_exists::<EmulateCapabilities>),
                    send_key_events_no_emulation
                        .run_if(not(resource_exists::<EmulateCapabilities>)),
                )
                    .in_set(InputSet::EmitBevy),
            );
    }
}

bitflags::bitflags! {
    /// Defines the capabilities of the terminal. Used to represet both detection
    /// ([`DetectedCapabilities`]) and emulation ([`EmulationPolicy`]).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct Capabilities: u8 {
        /// Represents a terminal that emits its own key release events.
        const KEY_RELEASE = 0b0000_0001;
        /// Represents a terminal that emits its own modifier key press and release events
        /// independently from other keys.
        const MODIFIER = 0b0000_0010;
        const ALL = Self::KEY_RELEASE.bits() | Self::MODIFIER.bits();
    }
}

/// Keyboard emulation policy
///
/// - The [`Automatic`][EmulationPolicy::Automatic] policy will emulate key release or modifiers if
///   they have not been detected.
///
/// - The [`Manual`][EmulationPolicy::Manual] policy defines whether modifiers or key releases will
///   be emulated.
///
/// Note: Any detected capabilities will not be emulated.
#[derive(Debug, Default, Resource, Clone, Copy)]
pub enum EmulationPolicy {
    /// Emulate everything that has not been detected.
    #[default]
    Automatic,
    /// Define what will be emulated.
    Manual(Capabilities),
}

impl EmulationPolicy {
    /// Return which capabilities to emulate based on the detected capabilities.
    pub fn emulate_capabilities(&self, detected: &DetectedCapabilities) -> Capabilities {
        match self {
            EmulationPolicy::Automatic => detected.complement(),
            EmulationPolicy::Manual(capabilties) => *capabilties - **detected,
        }
    }
}

/// The currently detected capabilities of the terminal.
///
/// When the application starts, this will be empty `detected.is_empty()` returns true. When a key
/// is pressed and released, if a key release event was emitted, then `detected` will contain
/// [Capability::KEY_RELEASE].
///
/// Likewise for a modifier key, if any modifier events are detected, then `detected` will contain
/// [Capability::MODIFIER].
///
/// Once those flags are set, they are never unset.
#[derive(Debug, Resource, Default, Deref, DerefMut)]
pub struct DetectedCapabilities(pub Capabilities);

/// A new type so we can implement Default and use with `Local`.
#[derive(Debug, Deref, DerefMut)]
struct Modifiers(KeyModifiers);

impl Default for Modifiers {
    fn default() -> Self {
        Self(KeyModifiers::empty())
    }
}

/// This is a double-buffer for the last keys pressed.
///
/// During processing we read from `current_frame` and write to `next_frame`.
#[derive(Default)]
struct LastPressedKeys {
    current_frame: HashSet<KeyboardInput>,
    next_frame: HashSet<KeyboardInput>,
}

impl LastPressedKeys {
    fn swap(&mut self) {
        std::mem::swap(&mut self.current_frame, &mut self.next_frame);
    }
}

/// If the terminal does not support sending key release events, this plugin will emulate them. A
/// crossterm event stream might look like this:
///
/// `Press A, Press B`
///
/// which will result in bevy events, i.e., a key release will be emitted on the next press key
/// event.
///
/// `Press A, Release A, Press B`
///
/// `A` is released when `B` is pressed, but how does one deal with releasing `B`? If another key is
/// pressed, that will release `B`. [ReleaseKey] concerns itself with how to handle the last pressed
/// key assuming it will be a while before another key press comes.
///
/// This plugin will emit events like so depending on the variant:
///
/// - Duration
///
///   `Press A, Release A, Press B, (timer finishes) Release B`
///
///   The default is a one second duration before emitting a release on the last pressed key
///
/// - FrameCount
///
///   `Press A, Release A, Press B, (frame count reached) Release B`
///
/// - Immediate
///
///   `Press A, Release A, Press B, (next frame) Release B`
///
/// - OnNextKey
///
///   `Press A, Release A, Press B`
///
///   The `Release B` event won't come until another key is pressed. This will make it look like the
///   last key pressed is being held down according to the bevy API.
#[derive(Resource, Debug)]
pub enum KeyReleaseMode {
    /// Release key after a short duration.
    Duration(Duration),
    /// Release key after a number of frames.
    FrameCount(u32),
    /// Release key next frame.
    Immediate,
    /// Do not emit a key release until someone presses another key.
    OnNextKey,
}

impl Default for KeyReleaseMode {
    /// Set the release key timer for 1 second by default.
    fn default() -> Self {
        KeyReleaseMode::Duration(Duration::from_secs(1))
    }
}

#[derive(Debug, Resource, Default)]
struct KeyReleaseTimer {
    mode: KeyReleaseMode,
    timer: Option<Timer>,
    frame_count: u32,
    finished: bool,
    // #[default]
    // None,
    // Count(u32),
    // Timer(Timer),
}

impl KeyReleaseTimer {
    fn finished(&self) -> bool {
        self.finished
    }

    fn tick(&mut self, delta: Duration) {
        match self.mode {
            KeyReleaseMode::Duration(_) => {
                if let Some(ref mut timer) = self.timer {
                    timer.tick(delta);
                    self.finished = timer.finished();
                }
            }
            KeyReleaseMode::FrameCount(count) => {
                self.frame_count = self.frame_count.saturating_add(1);
                self.finished = self.frame_count >= count;
            }
            KeyReleaseMode::Immediate => self.finished = true,
            KeyReleaseMode::OnNextKey => self.finished = false,
        }
    }

    fn reset(&mut self) {
        match self.mode {
            KeyReleaseMode::Duration(duration) => {
                self.timer = Some(Timer::new(duration, TimerMode::Once));
            }
            KeyReleaseMode::FrameCount(_) => self.frame_count = 0,
            KeyReleaseMode::Immediate => {}
            KeyReleaseMode::OnNextKey => {}
        }
        self.finished = false;
    }
}

/// Marker resource used to determine whether this plugin will emulate any terminal capabilities. If
/// it is not present, that's the best case because the terminal doesn't require this plugin to
/// emulate any capabilities. It's a simpler and faster code path.
#[derive(Debug, Resource, Default)]
pub struct EmulateCapabilities;

fn detect_capabilities(
    policy: Res<EmulationPolicy>,
    mut keys: EventReader<KeyEvent>,
    mut detected: ResMut<DetectedCapabilities>,
    mut commands: Commands,
) {
    if policy.is_changed() {
        commands.insert_resource(EmulateCapabilities);
        *detected = DetectedCapabilities::default();
    } else if policy.emulate_capabilities(&detected).is_empty() {
        // We don't need to emulate anything, so don't.
        commands.remove_resource::<EmulateCapabilities>();
    }
    for key_event in keys.read() {
        if matches!(key_event.code, crossterm::event::KeyCode::Modifier(_)) {
            **detected |= Capabilities::MODIFIER;
        }
        if matches!(key_event.kind, crossterm::event::KeyEventKind::Release) {
            **detected |= Capabilities::KEY_RELEASE;
        }
    }
}

fn emulate_release_keys(
    time: Res<Time>,
    window: Query<Entity, With<DummyWindow>>,
    emulation_policy: Res<EmulationPolicy>,
    detected_capabilities: Res<DetectedCapabilities>,
    mut key_release_timer: ResMut<KeyReleaseTimer>,
    mut crossterm_keys: EventReader<KeyEvent>,
    mut last_pressed: Local<LastPressedKeys>,
    mut bevy_keys: EventWriter<KeyboardInput>,
) {
    let emulation = emulation_policy.emulate_capabilities(&detected_capabilities);
    if !emulation.contains(Capabilities::KEY_RELEASE) {
        return;
    }
    key_release_timer.tick(time.delta());
    if !key_release_timer.finished() && crossterm_keys.is_empty() {
        // only release keys when either the timer has finished or a key has been pressed.
        return;
    }
    for crossterm_key in crossterm_keys.read() {
        if let Some((bevy_key, _, _)) = key_event_to_bevy(&crossterm_key, window.single()) {
            if last_pressed.current_frame.contains(&bevy_key) {
                // the key is being held down, so we shouldn't release it.
                last_pressed.current_frame.remove(&bevy_key);
            }
            last_pressed.next_frame.insert(bevy_key);
        }
    }
    for bevy_key in last_pressed.current_frame.drain() {
        let state = match bevy_key.state {
            ButtonState::Pressed => ButtonState::Released, // a key was pressed
            ButtonState::Released => ButtonState::Pressed, // a repeated key was "released"
        };
        bevy_keys.send(KeyboardInput { state, ..bevy_key });
    }
}

fn emulate_release_modifiers(
    window: Query<Entity, With<DummyWindow>>,
    emulation_policy: Res<EmulationPolicy>,
    detected_capabilities: Res<DetectedCapabilities>,
    key_release_timer: Res<KeyReleaseTimer>,
    mut modifiers: Local<Modifiers>,
    mut bevy_keys: EventWriter<KeyboardInput>,
) {
    let emulation = emulation_policy.emulate_capabilities(&detected_capabilities);
    if !emulation.contains(Capabilities::MODIFIER) {
        return;
    }
    // calling key_release_timer.tick() is not needed as we tick it in emulate_release_keys
    if !key_release_timer.finished() {
        return;
    }
    let window = window.single();
    for modifier in **modifiers {
        let state = ButtonState::Released;
        let modifier_event =
            modifier_to_bevy(crossterm_modifier_to_bevy_key(modifier), state, window);
        bevy_keys.send(modifier_event);
    }
    **modifiers = KeyModifiers::empty();
}

#[allow(clippy::too_many_arguments)] // TODO: simplify
fn emulate_key_events(
    window: Query<Entity, With<DummyWindow>>,
    mut crossterm_keys: EventReader<KeyEvent>,
    mut modifiers: Local<Modifiers>,
    mut last_pressed: Local<LastPressedKeys>,
    mut bevy_keys: EventWriter<KeyboardInput>,
    mut key_release_timer: Local<KeyReleaseTimer>,
    detected_capabilities: Res<DetectedCapabilities>,
    emulation_policy: Res<EmulationPolicy>,
) {
    if crossterm_keys.is_empty() {
        return;
    }

    let window = window.single();
    let emulation = emulation_policy.emulate_capabilities(&detected_capabilities);
    for key_event in crossterm_keys.read() {
        if let Some((bevy_key, key_modifiers, is_repeated)) = key_event_to_bevy(key_event, window) {
            if emulation.contains(Capabilities::MODIFIER) && key_modifiers != **modifiers {
                send_modifier_keys(key_modifiers, &mut modifiers, window, &mut bevy_keys);
            }

            // Repeated key events are converted to key release events by `key_event_to_bevy()`.
            // But are queued up to emit a key press on the next frame.
            if is_repeated {
                last_pressed.next_frame.insert(bevy_key.clone());
            }

            last_pressed.next_frame.insert(bevy_key.clone());
            bevy_keys.send(bevy_key);
        }
    }
    for key in last_pressed.current_frame.drain() {
        // In general this is where we emit key released events. However, we also emit key pressed
        // events for repeated keys.
        let reciprocal_event = KeyboardInput {
            state: match key.state {
                ButtonState::Pressed => ButtonState::Released,
                ButtonState::Released => ButtonState::Pressed,
            },
            ..key
        };
        bevy_keys.send(reciprocal_event);
    }
    last_pressed.swap();
    key_release_timer.reset();
}

fn send_modifier_keys(
    key_modifiers: KeyModifiers,
    modifiers: &mut Local<Modifiers>,
    window: Entity,
    bevy_keys: &mut EventWriter<KeyboardInput>,
) {
    for modifier in key_modifiers.symmetric_difference(***modifiers) {
        let button_state = if key_modifiers.contains(modifier) {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        };
        let bevy_key = crossterm_modifier_to_bevy_key(modifier);
        let event = modifier_to_bevy(bevy_key, button_state, window);
        bevy_keys.send(event);
    }
    ***modifiers = key_modifiers;
}

/// Send bevy events without any emulation. This merely shows how simple life is
/// when emulation is not involved.
fn send_key_events_no_emulation(
    mut keys: EventReader<KeyEvent>,
    window: Query<Entity, With<DummyWindow>>,
    mut keyboard_input: EventWriter<KeyboardInput>,
    mut key_repeat_queue: Local<Vec<KeyboardInput>>,
) {
    for bevy_event in key_repeat_queue.drain(..) {
        keyboard_input.send(bevy_event);
    }
    let bevy_window = window.single();
    for key_event in keys.read() {
        if let Some((bevy_event, _modifiers, repeated)) = key_event_to_bevy(key_event, bevy_window)
        {
            if repeated {
                key_repeat_queue.push(KeyboardInput {
                    state: ButtonState::Pressed,
                    ..bevy_event.clone()
                });
            }
            keyboard_input.send(bevy_event);
        }
    }
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
        c::BackTab => {
            mods |= m::SHIFT;
            Some(b::Tab)
        }
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
        c::BackTab => {
            // mods |= m::SHIFT;
            Some(b::Tab)
        }
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
