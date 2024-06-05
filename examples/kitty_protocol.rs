use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    prelude::*,
};
use bevy_ratatui::{
    error::exit_on_error, event::KeyEvent, terminal::RatatuiContext, RatatuiPlugins,
};

fn main() {
    let wait_duration = std::time::Duration::from_secs_f64(1. / 60.); // 60 FPS
    App::new()
        .init_resource::<LastKeypress>()
        .add_plugins(RatatuiPlugins)
        .add_plugins(ScheduleRunnerPlugin::run_loop(wait_duration))
        .add_systems(PostStartup, setup_kitty_system)
        .add_systems(PreUpdate, keyboard_input_system)
        .add_systems(Update, draw_scene_system.pipe(exit_on_error))
        .run();
}

#[derive(Resource)]
struct KittyEnabled;

#[derive(Resource, Default)]
struct LastKeypress(pub Option<KeyEvent>);

fn setup_kitty_system(mut commands: Commands, mut ratatui: ResMut<RatatuiContext>) {
    if ratatui.enable_kitty_protocol().is_ok() {
        commands.insert_resource(KittyEnabled);
    }
}

fn draw_scene_system(
    mut context: ResMut<RatatuiContext>,
    kitty_enabled: Option<Res<KittyEnabled>>,
    last_keypress: Res<LastKeypress>,
) -> color_eyre::Result<()> {
    context.draw(|frame| {
        let mut text = ratatui::text::Text::raw(match kitty_enabled {
            Some(_) => "Kitty protocol enabled!",
            None => "Kitty protocol not supported in this terminal.",
        });

        text.push_line("Press any key. Press 'q' to Quit.");

        if let Some(ref key_event) = last_keypress.0 {
            let code_string = format!("{:?}", key_event.code);
            let kind_string = match key_event.kind {
                crossterm::event::KeyEventKind::Press => "pressed",
                crossterm::event::KeyEventKind::Repeat => "repeated",
                crossterm::event::KeyEventKind::Release => "released",
            };
            text.push_line("");
            text.push_line(format!("{code_string} key was {kind_string}!"));
        }

        frame.render_widget(text.centered(), frame.size())
    })?;
    Ok(())
}

fn keyboard_input_system(
    mut events: EventReader<KeyEvent>,
    mut exit: EventWriter<AppExit>,
    mut last_keypress: ResMut<LastKeypress>,
) {
    use crossterm::event::KeyCode;
    for event in events.read() {
        match event.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                exit.send(AppExit);
            }
            _ => {
                last_keypress.0 = Some(event.clone());
            }
        }
    }
}
