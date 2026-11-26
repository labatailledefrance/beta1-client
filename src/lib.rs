//! La Bataille de France, beta 1 : client navigateur. Il affiche l'état reçu
//! du serveur et envoie des ordres ; il ne simule rien (F-01).
//!
//! - [`state`] : ce que le client sait du serveur (logique pure, testée en natif) ;
//! - `web` et `app` : WebSocket du navigateur et application Bevy (wasm32 seulement).

pub mod state;

// bevy est déclaré pour toutes les cibles (les derive de Bevy le cherchent là),
// mais seul le code wasm32 s'en sert.
#[cfg(not(target_arch = "wasm32"))]
use bevy as _;

#[cfg(target_arch = "wasm32")]
mod app;
#[cfg(target_arch = "wasm32")]
mod web;

/// Point d'entrée appelé par la page.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    app::run();
}
