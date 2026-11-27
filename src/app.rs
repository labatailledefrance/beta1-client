//! Application Bevy du navigateur (squelette) : ligne d'état du serveur et
//! ligne d'état des données, dans la police de l'interface chargée depuis la
//! publication d'assets.

use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;
use bevy::text::{FontSize, FontSmoothing};
use bevy::window::WindowPlugin;

use crate::donnees::{self, EtatDonnees, Publications};
use crate::state::{ClientState, hello};
use crate::web::{Connection, NetEvent, Requetes, server_url};

#[derive(Resource, Default)]
struct StateRes(ClientState);

#[derive(Resource, Default)]
struct DonneesRes(EtatDonnees);

/// Police de l'interface, une fois sa publication connue.
#[derive(Resource)]
struct Police(Handle<Font>);

/// Requêtes HTTP en cours.
#[derive(Clone, Copy)]
enum Requete {
    Publications,
    ManifesteTiles,
    ManifesteAssets,
}

#[derive(Component)]
struct StatusLine;

#[derive(Component)]
struct DataLine;

/// Lance l'application dans le canvas `#jeu` de la page.
pub fn run() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "La Bataille de France".into(),
                    canvas: Some("#jeu".into()),
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            })
            // Racine web « assets/ » (défaut) : /assets/<id>/… est la
            // publication d'assets du serveur de données. Pas de fichiers .meta.
            .set(AssetPlugin { meta_check: AssetMetaCheck::Never, ..default() }),
    )
    .init_resource::<StateRes>()
    .init_resource::<DonneesRes>()
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        ((poll_network, update_status_line).chain(), (poll_requetes, update_data_line).chain(), apply_font),
    );
    match Connection::open(&server_url(), hello()) {
        Ok(c) => {
            app.insert_non_send(c);
        }
        Err(e) => error!("WebSocket : {e:?}"),
    }
    let requetes = Requetes::<Requete>::new();
    requetes.get(Requete::Publications, donnees::URL_PUBLICATIONS);
    app.insert_non_send(requetes);
    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        StatusLine,
        Text::new(""),
        Node { position_type: PositionType::Absolute, top: px(12), left: px(12), ..default() },
    ));
    commands.spawn((
        DataLine,
        Text::new(""),
        Node { position_type: PositionType::Absolute, top: px(36), left: px(12), ..default() },
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

fn poll_requetes(
    mut commands: Commands,
    requetes: NonSend<Requetes<Requete>>,
    assets: Res<AssetServer>,
    mut etat: ResMut<DonneesRes>,
) {
    for (cle, r) in requetes.drain() {
        let texte = match r {
            Ok(t) => t,
            Err(e) => {
                warn!("données : {e}");
                etat.0.erreur = Some(e);
                continue;
            }
        };
        match cle {
            Requete::Publications => match Publications::lire(&texte) {
                Ok(p) => {
                    requetes.get(Requete::ManifesteTiles, &p.url_tiles("MANIFESTE"));
                    requetes.get(Requete::ManifesteAssets, &p.url_assets("MANIFESTE"));
                    commands.insert_resource(Police(assets.load(p.chemin_asset(donnees::POLICE))));
                    info!("publications : {} et {}", p.tiles, p.assets);
                    etat.0.publications = Some(p);
                }
                Err(e) => etat.0.erreur = Some(e),
            },
            Requete::ManifesteTiles => etat.0.departements = Some(donnees::departements(&texte)),
            Requete::ManifesteAssets => etat.0.fichiers = Some(donnees::fichiers(&texte)),
        }
    }
}

fn update_data_line(etat: Res<DonneesRes>, mut lines: Query<&mut Text, With<DataLine>>) {
    if etat.is_changed() {
        for mut t in &mut lines {
            t.0 = etat.0.ligne();
        }
    }
}

/// Police matricielle chargée : appliquée à tout le texte, sans lissage (un
/// pixel de la police = un pixel de l'écran à 16 px).
fn apply_font(
    police: Option<Res<Police>>,
    assets: Res<AssetServer>,
    mut textes: Query<&mut TextFont>,
    mut faite: Local<bool>,
) {
    let Some(police) = police else { return };
    if *faite || !assets.is_loaded(&police.0) {
        return;
    }
    for mut f in &mut textes {
        *f = TextFont::from_font_size(FontSize::Px(16.0)).with_font(police.0.clone());
        f.font_smoothing = FontSmoothing::None;
    }
    *faite = true;
}
