use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    input::{keyboard::KeyboardInput, ButtonState},
    prelude::*,
};
use bevy_ratatui::{
    error::exit_on_error,
    event::KeyEvent,
    input::{
        Capabilities, DetectedCapabilities, EmulateCapabilities, EmulationPolicy, KeyReleaseMode,
    },
    kitty::KittyEnabled,
    terminal::RatatuiContext,
    RatatuiPlugins,
};
use crossterm::event::KeyEventKind;
use ratatui::text::Text;

fn main() {
    let wait_duration = std::time::Duration::from_secs_f64(1. / 60.); // 60 FPS
    App::new()
        .add_plugins(RatatuiPlugins::default())
        .add_plugins(ScheduleRunnerPlugin::run_loop(wait_duration))
        .add_systems(
            PreUpdate,
            (
                keyboard_input_system,
                bevy_keyboard_input_system,
                bevy_keypresses,
            ),
        )
        .add_systems(Update, draw_scene_system.pipe(exit_on_error))
        .add_systems(Update, hotkeys)
        .run();
}

#[derive(Resource, Deref, DerefMut)]
struct LastKeypress(pub KeyEvent);

#[derive(Resource, Deref, DerefMut)]
struct LastBevyKeypress(pub KeyboardInput);

#[derive(Resource, Deref, DerefMut)]
struct BevyKeypresses(pub Vec<KeyCode>);

#[allow(clippy::too_many_arguments)] // TODO: simplify
fn draw_scene_system(
    mut context: ResMut<RatatuiContext>,
    kitty_enabled: Option<Res<KittyEnabled>>,
    last_keypress: Option<Res<LastKeypress>>,
    last_bevy_keypress: Option<Res<LastBevyKeypress>>,
    bevy_keypresses: Option<Res<BevyKeypresses>>,
    emulate: Option<Res<EmulateCapabilities>>,
    detected: Res<DetectedCapabilities>,
    release_key: Res<KeyReleaseMode>,
    policy: Res<EmulationPolicy>,
) -> color_eyre::Result<()> {
    context.draw(|frame| {
        let mut text = Text::raw(if kitty_enabled.is_some() {
            "Kitty protocol enabled!"
        } else {
            "Kitty protocol not supported in this terminal."
        });

        text.push_line(match **detected {
            Capabilities::ALL => "Detected modifiers and key release.",
            Capabilities::KEY_RELEASE => "Detected key release but not modifiers.",
            Capabilities::MODIFIER => "Detected modifiers but not key release.",
            _ => "Did not detect modifiers or key release.",
        });
        text.push_line(match policy.emulate_capabilities(&detected) {
            Capabilities::ALL => "Emulate modifiers and key release.",
            Capabilities::KEY_RELEASE => "Emulate key release.",
            Capabilities::MODIFIER => "Emulate modifiers.",
            _ => "Do not emulate modifiers or key release.",
        });
        text.push_line(if emulate.is_some() {
            "Emulate marker is present."
        } else {
            "Emulate marker is not present."
        });
        text.push_line(format!(
            "Press 'r' to cycle release key policy, currently {:?}",
            release_key
        ));

        text.push_line("Press any key. Press 'q' to Quit.");

        if let Some(key_press) = last_keypress {
            let code_string = format!("{:?}", key_press.code);
            let kind_string = match key_press.kind {
                KeyEventKind::Press => "pressed",
                KeyEventKind::Repeat => "repeated",
                KeyEventKind::Release => "released",
            };
            text.push_line("");
            text.push_line(format!("{code_string} key was {kind_string}!"));
        }

        if let Some(key_press) = last_bevy_keypress {
            let code_string = format!("{:?}", key_press.key_code);
            let kind_string = match key_press.state {
                ButtonState::Pressed => "pressed",
                ButtonState::Released => "released",
            };
            text.push_line("");
            text.push_line(format!("bevy {code_string} key was {kind_string}!"));
        }

        if let Some(key_presses) = bevy_keypresses {
            text.push_line("");
            for key_press in &key_presses.0 {
                let code_string = format!("{:?}", key_press);
                text.push_line(format!("bevy {code_string} key is pressed!"));
            }
        }

        frame.render_widget(text.centered(), frame.size())
    })?;
    Ok(())
}

fn hotkeys(
    input: Res<ButtonInput<KeyCode>>,
    mut exit: EventWriter<AppExit>,
    mut release_key: ResMut<KeyReleaseMode>,
    mut policy: ResMut<EmulationPolicy>,
) {
    use bevy::input::keyboard::KeyCode::*;
    if input.just_pressed(KeyQ) | input.just_pressed(Escape) {
        exit.send_default();
    } else if input.just_pressed(KeyR) {
        use KeyReleaseMode::*;
        *release_key = match *release_key {
            OnNextKey => FrameCount(60),
            FrameCount(_) => Duration(std::time::Duration::from_secs(2)),
            Duration(_) => Immediate,
            Immediate => OnNextKey,
        };
    } else if input.just_pressed(KeyP) {
        // Mutate the policy to ensure that the Emulate marker is removed
        // (however briefly).
        #[allow(clippy::self_assignment)]
        {
            *policy = *policy;
        }
    }
}

fn keyboard_input_system(mut events: EventReader<KeyEvent>, mut commands: Commands) {
    for event in events.read() {
        commands.insert_resource(LastKeypress(event.clone()));
    }
}

fn bevy_keyboard_input_system(mut events: EventReader<KeyboardInput>, mut commands: Commands) {
    for event in events.read() {
        commands.insert_resource(LastBevyKeypress(event.clone()));
    }
}

fn bevy_keypresses(keys: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    commands.insert_resource(BevyKeypresses(keys.get_pressed().cloned().collect()));
}
