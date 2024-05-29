use core::panic;
use std::time::Duration;

use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    core::FrameCount,
    prelude::*,
};
use bevy_ratatui::{
    error::exit_on_error, event::KeyEvent, terminal::RatatuiContext, RatatuiPlugins,
};
use crossterm::event::KeyCode;
use ratatui::{
    buffer,
    layout::Rect,
    style::{Color, Stylize},
    text::Line,
    widgets::WidgetRef,
};

fn main() {
    let frame_rate = Duration::from_secs_f64(1. / 60.);
    App::new()
        .add_plugins(RatatuiPlugins)
        .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(frame_rate)))
        .add_systems(PreUpdate, keyboard_input_system.pipe(exit_on_error))
        .add_systems(Update, ui_system.pipe(exit_on_error))
        .init_resource::<Counter>()
        .add_event::<CounterEvent>()
        .init_state::<AppState>()
        .add_systems(Update, update_counter_system)
        .run();
}

fn ui_system(
    mut context: ResMut<RatatuiContext>,
    frame_count: Res<FrameCount>,
    counter: Res<Counter>,
    app_state: Res<State<AppState>>,
) -> color_eyre::Result<()> {
    context.draw(|frame| {
        let area = frame.size();
        let frame_count = Line::from(format!("Frame Count: {}", frame_count.0)).right_aligned();

        frame.render_widget(frame_count, area);

        frame.render_widget(counter.as_ref(), area);
        frame.render_widget(app_state.get(), area)
    })?;
    Ok(())
}

fn keyboard_input_system(
    mut events: EventReader<KeyEvent>,
    mut app_exit: EventWriter<AppExit>,
    mut counter_events: EventWriter<CounterEvent>,
) -> color_eyre::Result<()> {
    for event in events.read() {
        match event.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                app_exit.send_default();
            }
            KeyCode::Char('p') => {
                panic!("Panic!");
            }
            KeyCode::Left => {
                counter_events.send(CounterEvent::Decrement);
            }
            KeyCode::Right => {
                counter_events.send(CounterEvent::Increment);
            }
            _ => {}
        }
    }
    Ok(())
}

#[derive(Default, Resource, Debug, Deref, DerefMut)]
struct Counter(i32);

impl Counter {
    fn decrement(&mut self) {
        self.0 -= 1;
    }

    fn increment(&mut self) {
        self.0 += 1;
    }
}

#[derive(Debug, Clone, Copy, Event, PartialEq, Eq)]
enum CounterEvent {
    Increment,
    Decrement,
}

fn update_counter_system(
    mut counter: ResMut<Counter>,
    mut events: EventReader<CounterEvent>,
    mut app_state: ResMut<NextState<AppState>>,
) {
    for event in events.read() {
        match event {
            CounterEvent::Increment => counter.increment(),
            CounterEvent::Decrement => counter.decrement(),
        }
        // demonstrates changing something in the app state based on the counter value
        if counter.0 < 0 {
            app_state.set(AppState::Negative);
        } else {
            app_state.set(AppState::Positive);
        }
    }
}

impl WidgetRef for Counter {
    fn render_ref(&self, area: Rect, buf: &mut buffer::Buffer) {
        let counter = format!("Counter: {}", self.0);
        Line::from(counter).render_ref(area, buf);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
enum AppState {
    Negative,
    #[default]
    Positive,
}

impl WidgetRef for AppState {
    fn render_ref(&self, area: Rect, buf: &mut buffer::Buffer) {
        let (state, color) = match self {
            AppState::Negative => ("Negative", Color::Red),
            AppState::Positive => ("Positive", Color::Green),
        };
        Line::from(state).centered().bg(color).render_ref(area, buf);
    }
}
