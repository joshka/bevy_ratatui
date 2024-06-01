# Bevy_ratatui

An experiment using Ratatui from within a Bevy app. Note that this library is explicitly unstable
and will break compatibility between 0.x versions.

The goal of this (at least to start) is not to do any rendering / 3D / etc. in the terminal, but
just to see how useful it is to use the bevy concepts for apps. This may change over time.

## Example app

This app demonstrates:

- Using the `RatatuiContext` resource to draw widgets to the terminal
- ScheduleRunnerPlugin to run the app loop
- Using `Event`s to communicate
- Handling `State`s to add logic that runs on transition (e.g. changing the background color when
  going from Negative to Positive in a simple counter app)

```shell
cargo run --example demo
```

Keys:

- Left / Right - modify the counter (look at what happens when you go negative)
- Q / Esc - quit
- P - simulate a panic (tests the color_eyre panic hooks)

![Made with VHS](https://vhs.charm.sh/vhs-2g0S6RgGGQHseTCNItEQhg.gif)

## Ideas on progressing this

- [ ] Rewrite Ratatui's `Terminal` as a Bevy SubApp. It's possible that this would allow rendering
      to happen while the main app is continuing to run
- [ ] Consider how to handle layout. Bevy has a lot of code related to this which might be possible
      to incorporate
- [ ] Convert Crossterm events into the bevy standard
- [ ] Collab with the other bevy/crossterm/ratatui libs
  - <https://github.com/cxreiff/bevy_rat> - seems like the most recent / up to date crate with some
    fairly similar ideas. Has some stuff for rendering images to the screen (e.g. spinning 3D cube).
  - <https://github.com/octotep/bevy_crossterm> - Crossterm plugin for the bevy game engine
  - <https://github.com/TheEmeraldBee/widgetui> - A bevy systems like widget system for ratatui and
    crossterm. This is a bevy-like approach (not actual bevy) and has some neat ideas about Widgets
  - <https://github.com/AlephAlpha/roguelike-bevy-crossterm> - takes the approach of defining a
    custom runner to handle the event loop to make a roguelike game
  - <https://github.com/Mimea005/bevyterm> - A bevy renderer for the terminal using crossterm that
    does not use a custom runner like bevy_crossterm.
  - <https://github.com/gold-silver-copper/bevy_ratatui> - is sort of the opposite of this idea. It
    runs a Ratatui app using bevy as the backend to draw to a graphical window / webpage target
  - <https://github.com/gold-silver-copper/ratatui_egui_wasm> - continuation of the previous with a
    egui backend to render ratatui apps to the web
  - <https://github.com/sstelfox/bevy_tui> - tui-rs / bevy seems dead (last commit Jan 2023)

## Previous bevy_ratatui crate

Previously there was another crate using this name which has since migrated to
[ratatui_egui_wasm](https://github.com/gold-silver-copper/ratatui_egui_wasm).  A ratatui backend
that is also an egui widget. Deploy on web with WASM or ship natively with bevy, macroquad, or
eframe. Demo at <https://gold-silver-copper.github.io/>

## License

Copyright (c) Josh McKinney

This project is licensed under either of

- Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

See [CONTRIBUTING.md](CONTRIBUTING.md).
