//! Ce que le client sait du serveur : statut de la connexion et dernier tick.

use beta1_common::protocole::{DuClient, DuServeur, EMPREINTE, Raison};

/// Statut de la connexion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Statut {
    /// En attente de la réponse à la poignée de main.
    Connexion,
    /// Poignée de main acceptée.
    Connecte,
    /// Refusé par le serveur.
    Refuse(Raison),
    /// Connexion fermée.
    Ferme,
}

/// État connu du client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Etat {
    /// Statut de la connexion.
    pub statut: Statut,
    /// Dernier tick reçu.
    pub tick: Option<u64>,
    /// Messages invalides reçus (ignorés).
    pub invalides: u64,
}

impl Default for Etat {
    fn default() -> Self {
        Etat { statut: Statut::Connexion, tick: None, invalides: 0 }
    }
}

/// Premier message à envoyer.
pub fn bonjour() -> Vec<u8> {
    DuClient::Bonjour { empreinte: EMPREINTE }.encoder()
}

impl Etat {
    /// Applique un message du serveur.
    pub fn recevoir(&mut self, octets: &[u8]) {
        match (DuServeur::decoder(octets), self.statut) {
            (Ok(DuServeur::Bienvenue { tick }), Statut::Connexion) => {
                self.statut = Statut::Connecte;
                self.tick = Some(tick);
            }
            (Ok(DuServeur::Refus { raison }), Statut::Connexion) => self.statut = Statut::Refuse(raison),
            (Ok(DuServeur::Etat { tick }), Statut::Connecte) => self.tick = Some(tick),
            // Message valide hors de propos, ou invalide : ignoré, compté.
            _ => self.invalides += 1,
        }
    }
    /// Connexion fermée.
    pub fn fermer(&mut self) {
        if !matches!(self.statut, Statut::Refuse(_)) {
            self.statut = Statut::Ferme;
        }
    }
    /// Ligne d'état affichée.
    pub fn texte(&self) -> String {
        match self.statut {
            Statut::Connexion => "Connexion au serveur…".to_owned(),
            Statut::Connecte => match self.tick {
                Some(t) => format!("Connecté — tick {t} ({:.1} s de jeu)", t as f64 / 20.0),
                None => "Connecté".to_owned(),
            },
            Statut::Refuse(Raison::Version) => "Nouvelle version du jeu : rechargez la page.".to_owned(),
            Statut::Refuse(Raison::Plein) => "Serveur plein.".to_owned(),
            Statut::Refuse(Raison::Invalide) => "Refusé par le serveur.".to_owned(),
            Statut::Ferme => "Connexion perdue.".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poignee_de_main_puis_etats() {
        let mut e = Etat::default();
        assert_eq!(DuClient::decoder(&bonjour()), Ok(DuClient::Bonjour { empreinte: EMPREINTE }));
        e.recevoir(&DuServeur::Etat { tick: 3 }.encoder());
        assert_eq!((e.statut, e.tick, e.invalides), (Statut::Connexion, None, 1));
        e.recevoir(&DuServeur::Bienvenue { tick: 10 }.encoder());
        e.recevoir(&DuServeur::Etat { tick: 12 }.encoder());
        assert_eq!((e.statut, e.tick), (Statut::Connecte, Some(12)));
        assert!(e.texte().contains("tick 12"));
    }

    #[test]
    fn refus_de_version_garde_son_message() {
        let mut e = Etat::default();
        e.recevoir(&DuServeur::Refus { raison: Raison::Version }.encoder());
        e.fermer();
        assert_eq!(e.statut, Statut::Refuse(Raison::Version));
        assert!(e.texte().contains("rechargez"));
    }

    #[test]
    fn octets_invalides_ignores() {
        let mut e = Etat::default();
        e.recevoir(&[0xFF, 1, 2]);
        e.recevoir(&[]);
        assert_eq!((e.statut, e.invalides), (Statut::Connexion, 2));
    }
}
