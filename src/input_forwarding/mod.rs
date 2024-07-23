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
//! [bevy_keys](https://github.com/joshka/bevy_ratatui/tree/main/examples/bevy_keys.rs)
//! shows what keys crossterm has received and what bevy keys are emitted. In
//! addition it will show what capabilities have been detected and what
//! emulation is being used.
//!
//! This binary can be instructive in determining what capabilities a terminal
//! is setup to provide. (Some terminals require enabling the kitty protocol and
//! some terminals only support part of the protocol.)
//!
//! # Configuration
//!
//! There are two things one can configure on this plugin: the release key
//! timer, and the emulation policy. In order to explain those, it helps to have
//! a brief overview of what the terminal may or may not provide.
//!
//! ## Terminal
//!
//! Terminal input events are varied. A standard terminal for instance only
//! provides key press events and modifier keys are not given except in
//! conjunction with another key. Luckily extensions exist to make key handling
//! more comprehensive like the [kitty comprehensive keyboard handling
//! protocol](https://sw.kovidgoyal.net/kitty/keyboard-protocol/) that
//! bevy_ratatui can use.
//!
//! In order to provide a semblance of expected input for bevy, this plugin can
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
//! can emulate them. By default it will use a timeout to emit a release key
//! for the last key pressed. An event stream might look like this:
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
//! # use bevy_ratatui::input_forwarding::*;
//! # let mut app = App::new();
//! app.insert_resource(ReleaseKey::Duration(Duration::from_secs_f32(0.5)));
//! ```
//!
//! There are other policies one can choose by configuring [ReleaseKey]. See its
//! documentation for more details.
//!
//! # Terminal Choice
//!
//! For the best experience, it is recommended to enable the kitty protocol on
//! your terminal. [See
//! here](https://sw.kovidgoyal.net/kitty/keyboard-protocol/) for a list of
//! terminals implementing this protocol.
mod keyboard;
pub use keyboard::*;
