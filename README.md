# Bevy_ratatui

An experiment using Ratatui from within a Bevy app.

The goal of this was not to do any rendering / 3D / etc. in the terminal, but just to see how useful
it is to use the bevy concepts for apps.

Demonstrates:

- Resource for the terminal (wrapped in a `Context` struct)
- ScheduleRunnerPlugin to run the app loop
- Events to communicate
- States to handle changing from one state to another (e.g. positive to negative)

Run the example app:

```shell
cargo run --example demo
```

Keys:

- Left / Right - modify the counter (look at what happens when you go negative)
- Q / Esc - quit
- P - simulate a panic (tests the color_eyre panic hooks)

![Made with VHS](https://vhs.charm.sh/vhs-2g0S6RgGGQHseTCNItEQhg.gif)
