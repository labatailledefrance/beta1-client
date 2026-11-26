//! La Bataille de France, beta 1 : client navigateur. Il affiche l'état reçu
//! du serveur et envoie des ordres ; il ne simule rien (F-01).
//!
//! - [`etat`] : ce que le client sait du serveur (logique pure, testée en natif) ;
//! - `web` et `jeu` : WebSocket du navigateur et application Bevy (wasm32 seulement).

pub mod etat;

#[cfg(target_arch = "wasm32")]
mod jeu;
#[cfg(target_arch = "wasm32")]
mod web;

/// Point d'entrée appelé par la page.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn demarrer() {
    jeu::lancer();
}
