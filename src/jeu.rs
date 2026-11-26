//! Application Bevy du navigateur (squelette) : une ligne d'état.

use bevy::prelude::*;
use bevy::window::WindowPlugin;

use crate::etat::{Etat, bonjour};
use crate::web::{Evenement, Lien, adresse};

#[derive(Resource, Default)]
struct EtatRes(Etat);

#[derive(Component)]
struct Ligne;

/// Lance l'application dans le canevas `#jeu` de la page.
pub fn lancer() {
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
    .init_resource::<EtatRes>()
    .add_systems(Startup, preparer)
    .add_systems(Update, (lire, afficher).chain());
    match Lien::ouvrir(&adresse(), bonjour()) {
        Ok(l) => {
            app.insert_non_send(l);
        }
        Err(e) => error!("WebSocket : {e:?}"),
    }
    app.run();
}

fn preparer(mut commandes: Commands) {
    commandes.spawn(Camera2d);
    commandes.spawn((
        Ligne,
        Text::new(""),
        Node { position_type: PositionType::Absolute, top: px(12), left: px(12), ..default() },
    ));
}

fn lire(lien: Option<NonSend<Lien>>, mut etat: ResMut<EtatRes>) {
    let Some(lien) = lien else { return };
    for e in lien.vider() {
        match e {
            Evenement::Message(b) => etat.0.recevoir(&b),
            Evenement::Ferme => etat.0.fermer(),
        }
    }
}

fn afficher(etat: Res<EtatRes>, mut lignes: Query<&mut Text, With<Ligne>>) {
    if etat.is_changed() {
        for mut t in &mut lignes {
            t.0 = etat.0.texte();
        }
    }
}
