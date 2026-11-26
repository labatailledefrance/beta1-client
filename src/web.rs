//! WebSocket du navigateur. Les rappels JavaScript remplissent une file que
//! l'application Bevy vide à chaque frame (tout se passe sur le fil principal).

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use js_sys::{ArrayBuffer, Uint8Array};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::{Closure, JsValue};
use web_sys::{BinaryType, MessageEvent, WebSocket};

/// Événement réseau.
pub enum Evenement {
    /// Message binaire reçu.
    Message(Vec<u8>),
    /// Connexion fermée.
    Ferme,
}

/// Rappels JavaScript, gardés vivants tant que la connexion existe.
struct Rappels {
    _message: Closure<dyn FnMut(MessageEvent)>,
    _ferme: Closure<dyn FnMut(JsValue)>,
    _ouvert: Closure<dyn FnMut(JsValue)>,
}

/// Connexion au serveur.
pub struct Lien {
    _ws: WebSocket,
    file: Rc<RefCell<VecDeque<Evenement>>>,
    _rappels: Rappels,
}

impl Lien {
    /// Ouvre la connexion ; `bonjour` est envoyé dès l'ouverture.
    pub fn ouvrir(url: &str, bonjour: Vec<u8>) -> Result<Lien, JsValue> {
        let ws = WebSocket::new(url)?;
        ws.set_binary_type(BinaryType::Arraybuffer);
        let file = Rc::new(RefCell::new(VecDeque::new()));

        let f = file.clone();
        let message = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
            if let Ok(b) = e.data().dyn_into::<ArrayBuffer>() {
                f.borrow_mut().push_back(Evenement::Message(Uint8Array::new(&b).to_vec()));
            }
        });
        let f = file.clone();
        let ferme = Closure::<dyn FnMut(JsValue)>::new(move |_| f.borrow_mut().push_back(Evenement::Ferme));
        let w = ws.clone();
        let ouvert = Closure::<dyn FnMut(JsValue)>::new(move |_| {
            let _ = w.send_with_u8_array(&bonjour);
        });
        ws.set_onmessage(Some(message.as_ref().unchecked_ref()));
        ws.set_onclose(Some(ferme.as_ref().unchecked_ref()));
        ws.set_onopen(Some(ouvert.as_ref().unchecked_ref()));
        Ok(Lien { _ws: ws, file, _rappels: Rappels { _message: message, _ferme: ferme, _ouvert: ouvert } })
    }

    /// Événements reçus depuis le dernier appel.
    pub fn vider(&self) -> Vec<Evenement> {
        self.file.borrow_mut().drain(..).collect()
    }
}

/// Adresse du serveur : `?serveur=ws://…` dans l'URL de la page, sinon `/ws`
/// sur le même hôte (wss derrière l'ingress).
pub fn adresse() -> String {
    let Some(l) = web_sys::window().map(|w| w.location()) else { return "ws://127.0.0.1:7425".to_owned() };
    if let Some(s) = l.search().ok().and_then(|q| web_sys::UrlSearchParams::new_with_str(&q).ok()?.get("serveur")) {
        return s;
    }
    let schema = if l.protocol().ok().as_deref() == Some("https:") { "wss" } else { "ws" };
    format!("{schema}://{}/ws", l.host().unwrap_or_default())
}
