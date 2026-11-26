//! Ce que le client sait du serveur : statut de la connexion et dernier tick.

use beta1_common::protocol::{ClientMessage, FINGERPRINT, RejectReason, ServerMessage, TICKS_PER_SECOND};

/// Statut de la connexion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// En attente de la réponse au handshake.
    Connecting,
    /// Handshake accepté.
    Connected,
    /// Rejeté par le serveur.
    Rejected(RejectReason),
    /// Connexion fermée.
    Closed,
}

/// État connu du client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientState {
    /// Statut de la connexion.
    pub status: ConnectionStatus,
    /// Dernier tick reçu.
    pub tick: Option<u64>,
    /// Messages invalides reçus (ignorés).
    pub invalid: u64,
}

impl Default for ClientState {
    fn default() -> Self {
        ClientState { status: ConnectionStatus::Connecting, tick: None, invalid: 0 }
    }
}

/// Premier message à envoyer.
pub fn hello() -> Vec<u8> {
    ClientMessage::Hello { fingerprint: FINGERPRINT }.encode()
}

impl ClientState {
    /// Applique un message du serveur.
    pub fn receive(&mut self, bytes: &[u8]) {
        match (ServerMessage::decode(bytes), self.status) {
            (Ok(ServerMessage::Welcome { tick }), ConnectionStatus::Connecting) => {
                self.status = ConnectionStatus::Connected;
                self.tick = Some(tick);
            }
            (Ok(ServerMessage::Rejected { reason }), ConnectionStatus::Connecting) => {
                self.status = ConnectionStatus::Rejected(reason)
            }
            (Ok(ServerMessage::State { tick }), ConnectionStatus::Connected) => self.tick = Some(tick),
            // Message valide hors de propos, ou invalide : ignoré, compté.
            _ => self.invalid += 1,
        }
    }
    /// Connexion fermée.
    pub fn close(&mut self) {
        if !matches!(self.status, ConnectionStatus::Rejected(_)) {
            self.status = ConnectionStatus::Closed;
        }
    }
    /// Ligne d'état affichée au joueur.
    pub fn status_line(&self) -> String {
        match self.status {
            ConnectionStatus::Connecting => "Connexion au serveur…".to_owned(),
            ConnectionStatus::Connected => match self.tick {
                Some(t) => format!("Connecté — tick {t} ({:.1} s de jeu)", t as f64 / f64::from(TICKS_PER_SECOND)),
                None => "Connecté".to_owned(),
            },
            ConnectionStatus::Rejected(RejectReason::Version) => {
                "Nouvelle version du jeu : rechargez la page.".to_owned()
            }
            ConnectionStatus::Rejected(RejectReason::Full) => "Serveur plein.".to_owned(),
            ConnectionStatus::Rejected(RejectReason::Invalid) => "Refusé par le serveur.".to_owned(),
            ConnectionStatus::Closed => "Connexion perdue.".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_puis_etats() {
        let mut s = ClientState::default();
        assert_eq!(ClientMessage::decode(&hello()), Ok(ClientMessage::Hello { fingerprint: FINGERPRINT }));
        s.receive(&ServerMessage::State { tick: 3 }.encode());
        assert_eq!((s.status, s.tick, s.invalid), (ConnectionStatus::Connecting, None, 1));
        s.receive(&ServerMessage::Welcome { tick: 10 }.encode());
        s.receive(&ServerMessage::State { tick: 12 }.encode());
        assert_eq!((s.status, s.tick), (ConnectionStatus::Connected, Some(12)));
        assert!(s.status_line().contains("tick 12"));
    }

    #[test]
    fn rejet_de_version_garde_son_message() {
        let mut s = ClientState::default();
        s.receive(&ServerMessage::Rejected { reason: RejectReason::Version }.encode());
        s.close();
        assert_eq!(s.status, ConnectionStatus::Rejected(RejectReason::Version));
        assert!(s.status_line().contains("rechargez"));
    }

    #[test]
    fn octets_invalides_ignores() {
        let mut s = ClientState::default();
        s.receive(&[0xFF, 1, 2]);
        s.receive(&[]);
        assert_eq!((s.status, s.invalid), (ConnectionStatus::Connecting, 2));
    }
}
