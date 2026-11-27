//! WebSocket du navigateur. Les callbacks JavaScript remplissent une queue que
//! l'application Bevy vide à chaque frame (tout se passe sur le main thread).

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use js_sys::{ArrayBuffer, Uint8Array};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::{Closure, JsValue};
use web_sys::{BinaryType, MessageEvent, WebSocket};

/// Événement réseau.
pub enum NetEvent {
    /// Message binaire reçu.
    Message(Vec<u8>),
    /// Connexion fermée.
    Closed,
}

/// Callbacks JavaScript, gardés vivants tant que la connexion existe.
struct Callbacks {
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_close: Closure<dyn FnMut(JsValue)>,
    _on_open: Closure<dyn FnMut(JsValue)>,
}

/// Connexion au serveur.
pub struct Connection {
    _ws: WebSocket,
    queue: Rc<RefCell<VecDeque<NetEvent>>>,
    _callbacks: Callbacks,
}

impl Connection {
    /// Ouvre la connexion ; `hello` est envoyé dès l'ouverture.
    pub fn open(url: &str, hello: Vec<u8>) -> Result<Connection, JsValue> {
        let ws = WebSocket::new(url)?;
        ws.set_binary_type(BinaryType::Arraybuffer);
        let queue = Rc::new(RefCell::new(VecDeque::new()));

        let q = queue.clone();
        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
            if let Ok(b) = e.data().dyn_into::<ArrayBuffer>() {
                q.borrow_mut().push_back(NetEvent::Message(Uint8Array::new(&b).to_vec()));
            }
        });
        let q = queue.clone();
        let on_close = Closure::<dyn FnMut(JsValue)>::new(move |_| q.borrow_mut().push_back(NetEvent::Closed));
        let w = ws.clone();
        let on_open = Closure::<dyn FnMut(JsValue)>::new(move |_| {
            let _ = w.send_with_u8_array(&hello);
        });
        ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));
        ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
        Ok(Connection {
            _ws: ws,
            queue,
            _callbacks: Callbacks { _on_message: on_message, _on_close: on_close, _on_open: on_open },
        })
    }

    /// Événements reçus depuis le dernier appel.
    pub fn drain(&self) -> Vec<NetEvent> {
        self.queue.borrow_mut().drain(..).collect()
    }
}

/// URL du serveur : `?server=ws://…` dans l'URL de la page, sinon `/ws` sur
/// le même hôte (wss derrière l'ingress).
pub fn server_url() -> String {
    let Some(location) = web_sys::window().map(|w| w.location()) else { return "ws://127.0.0.1:7425".to_owned() };
    if let Some(s) = location.search().ok().and_then(|q| web_sys::UrlSearchParams::new_with_str(&q).ok()?.get("server"))
    {
        return s;
    }
    let scheme = if location.protocol().ok().as_deref() == Some("https:") { "wss" } else { "ws" };
    format!("{scheme}://{}/ws", location.host().unwrap_or_default())
}

type Reponses<K> = Rc<RefCell<VecDeque<(K, Result<String, String>)>>>;
type Rappels = Rc<RefCell<Vec<Closure<dyn FnMut(JsValue)>>>>;

/// Requêtes HTTP GET vers le même hôte (`fetch` du navigateur). Comme pour le
/// WebSocket, les callbacks remplissent une queue vidée à chaque frame.
pub struct Requetes<K> {
    reponses: Reponses<K>,
    // Callbacks gardés vivants jusqu'à la fin de l'application : quelques
    // requêtes seulement (publications et manifestes).
    rappels: Rappels,
}

impl<K: Copy + 'static> Requetes<K> {
    pub fn new() -> Self {
        Requetes { reponses: Rc::default(), rappels: Rc::default() }
    }

    /// Lance `GET url` ; la réponse (texte, ou erreur) arrive avec la clé `cle`.
    pub fn get(&self, cle: K, url: &str) {
        let Some(window) = web_sys::window() else {
            self.reponses.borrow_mut().push_back((cle, Err("pas de fenêtre".to_owned())));
            return;
        };
        let (q, rappels) = (self.reponses.clone(), self.rappels.clone());
        let sur_reponse = Closure::<dyn FnMut(JsValue)>::new(move |r: JsValue| {
            let r = match r.dyn_into::<web_sys::Response>() {
                Ok(r) if r.ok() => r,
                Ok(r) => return q.borrow_mut().push_back((cle, Err(format!("HTTP {}", r.status())))),
                Err(_) => return q.borrow_mut().push_back((cle, Err("réponse invalide".to_owned()))),
            };
            let texte = match r.text() {
                Ok(p) => p,
                Err(e) => return q.borrow_mut().push_back((cle, Err(format!("{e:?}")))),
            };
            let q1 = q.clone();
            let sur_texte = Closure::<dyn FnMut(JsValue)>::new(move |t: JsValue| {
                q1.borrow_mut().push_back((cle, t.as_string().ok_or_else(|| "texte invalide".to_owned())));
            });
            let q2 = q.clone();
            let sur_erreur = Closure::<dyn FnMut(JsValue)>::new(move |e: JsValue| {
                q2.borrow_mut().push_back((cle, Err(format!("lecture : {e:?}"))));
            });
            let _ = texte.then2(&sur_texte, &sur_erreur);
            rappels.borrow_mut().extend([sur_texte, sur_erreur]);
        });
        let q = self.reponses.clone();
        let sur_echec = Closure::<dyn FnMut(JsValue)>::new(move |e: JsValue| {
            q.borrow_mut().push_back((cle, Err(format!("réseau : {e:?}"))));
        });
        let _ = window.fetch_with_str(url).then2(&sur_reponse, &sur_echec);
        self.rappels.borrow_mut().extend([sur_reponse, sur_echec]);
    }

    /// Réponses reçues depuis le dernier appel.
    pub fn drain(&self) -> Vec<(K, Result<String, String>)> {
        self.reponses.borrow_mut().drain(..).collect()
    }
}
