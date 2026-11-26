//! Application Bevy du navigateur (squelette) : une ligne d'état.

use bevy::prelude::*;
use bevy::window::WindowPlugin;

use crate::state::{ClientState, hello};
use crate::web::{Connection, NetEvent, server_url};

#[derive(Resource, Default)]
struct StateRes(ClientState);

#[derive(Component)]
struct StatusLine;

/// Lance l'application dans le canvas `#jeu` de la page.
pub fn run() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "La Bataille de France".into(),
            canvas: Some("#jeu".into()),
            fit_canvas_to_parent: true,
            ..default()
        }),
        ..default()
    }))
    .init_resource::<StateRes>()
    .add_systems(Startup, setup)
    .add_systems(Update, (poll_network, update_status_line).chain());
    match Connection::open(&server_url(), hello()) {
        Ok(c) => {
            app.insert_non_send(c);
        }
        Err(e) => error!("WebSocket : {e:?}"),
    }
    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        StatusLine,
        Text::new(""),
        Node { position_type: PositionType::Absolute, top: px(12), left: px(12), ..default() },
    ));
}

fn poll_network(connection: Option<NonSend<Connection>>, mut state: ResMut<StateRes>) {
    let Some(connection) = connection else { return };
    for e in connection.drain() {
        match e {
            NetEvent::Message(b) => state.0.receive(&b),
            NetEvent::Closed => state.0.close(),
        }
    }
}

fn update_status_line(state: Res<StateRes>, mut lines: Query<&mut Text, With<StatusLine>>) {
    if state.is_changed() {
        for mut t in &mut lines {
            t.0 = state.0.status_line();
        }
    }
}
