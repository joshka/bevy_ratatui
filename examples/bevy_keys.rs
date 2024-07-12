use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    input::{keyboard::KeyboardInput, ButtonState},
    prelude::*,
};
use bevy_ratatui::{
    error::exit_on_error, event::KeyEvent, kitty::KittyEnabled, terminal::RatatuiContext,
    RatatuiPlugins, bevy_compat::keyboard::{Capability, Detected, EmulationPolicy}
};
use crossterm::event::KeyEventKind;
use ratatui::text::Text;

fn main() {
    let wait_duration = std::time::Duration::from_secs_f64(1. / 60.); // 60 FPS
    App::new()
        .add_plugins(RatatuiPlugins {
            enable_input_forwarding: true,
            ..default()
        })
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
        .run();
}

#[derive(Resource, Deref, DerefMut)]
struct LastKeypress(pub KeyEvent);

#[derive(Resource, Deref, DerefMut)]
struct LastBevyKeypress(pub KeyboardInput);

#[derive(Resource, Deref, DerefMut)]
struct BevyKeypresses(pub Vec<KeyCode>);

fn draw_scene_system(
    mut context: ResMut<RatatuiContext>,
    kitty_enabled: Option<Res<KittyEnabled>>,
    last_keypress: Option<Res<LastKeypress>>,
    last_bevy_keypress: Option<Res<LastBevyKeypress>>,
    bevy_keypresses: Option<Res<BevyKeypresses>>,
    detected: Res<Detected>,
    policy: Res<EmulationPolicy>,
) -> color_eyre::Result<()> {
    context.draw(|frame| {
        let mut text = Text::raw(if kitty_enabled.is_some() {
            "Kitty protocol enabled!"
        } else {
            "Kitty protocol not supported in this terminal."
        });

        text.push_line(match detected.0 {
            Capability::ALL => "Detected modifiers and key release.",
            Capability::KEY_RELEASE => "Detected key release but not modifiers.",
            Capability::MODIFIER => "Detected modifiers but not key release.",
            _ => "Did not detect modifiers or key release.",
        });
        text.push_line(match policy.emulate_what(&detected) {
            Capability::ALL => "Emulate modifiers and key release.",
            Capability::KEY_RELEASE => "Emulate key release.",
            Capability::MODIFIER => "Emulate modifiers.",
            _ => "Do not emulate modifiers or key release.",
        });

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

fn keyboard_input_system(
    mut events: EventReader<KeyEvent>,
    mut exit: EventWriter<AppExit>,
    mut commands: Commands,
) {
    use crossterm::event::KeyCode;
    for event in events.read() {
        match event.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                exit.send_default();
            }
            _ => {
                commands.insert_resource(LastKeypress(event.clone()));
            }
        }
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
