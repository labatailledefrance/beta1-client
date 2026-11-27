//! Données du jeu servies par le serveur de données (H-06) : publication de
//! tiles et publication d'assets, chacune immuable et adressée par son
//! identifiant. `/publications.json` donne les deux chemins courants ; tout le
//! reste se lit sous ces chemins et se garde en cache sans expiration.
//!
//! Logique pure (lecture, validation, ligne d'état), testée en natif ; les
//! requêtes HTTP sont dans `web`.

use serde::Deserialize;

/// URL des publications courantes (même hôte que la page).
pub const URL_PUBLICATIONS: &str = "/publications.json";

/// Police de l'interface, dans la publication d'assets.
pub const POLICE: &str = "polices/DotGothic16-Regular.ttf";

/// Chemins des publications courantes, validés : `/tiles/<id>/`, `/assets/<id>/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Publications {
    pub tiles: String,
    pub assets: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Brut {
    tiles: String,
    assets: String,
}

/// `/<prefixe>/<id>/` avec `id` en `[a-z0-9-]`, 1 à 64 caractères : le reste
/// est refusé (une URL venue du réseau ne choisit ni l'hôte ni le chemin).
fn identifiant<'a>(chemin: &'a str, prefixe: &str) -> Option<&'a str> {
    let id = chemin.strip_prefix('/')?.strip_prefix(prefixe)?.strip_prefix('/')?.strip_suffix('/')?;
    let ok = !id.is_empty()
        && id.len() <= 64
        && !id.starts_with('-')
        && id.bytes().all(|o| o.is_ascii_lowercase() || o.is_ascii_digit() || o == b'-');
    ok.then_some(id)
}

impl Publications {
    pub fn lire(json: &str) -> Result<Publications, String> {
        let b: Brut = serde_json::from_str(json).map_err(|e| format!("publications.json illisible : {e}"))?;
        if identifiant(&b.tiles, "tiles").is_none() {
            return Err(format!("chemin de tiles refusé : {:?}", b.tiles));
        }
        if identifiant(&b.assets, "assets").is_none() {
            return Err(format!("chemin d'assets refusé : {:?}", b.assets));
        }
        Ok(Publications { tiles: b.tiles, assets: b.assets })
    }

    pub fn id_tiles(&self) -> &str {
        identifiant(&self.tiles, "tiles").unwrap_or_default()
    }

    pub fn id_assets(&self) -> &str {
        identifiant(&self.assets, "assets").unwrap_or_default()
    }

    /// Chemin pour l'AssetServer de Bevy, dont la racine web est `assets/`.
    pub fn chemin_asset(&self, relatif: &str) -> String {
        format!("{}/{relatif}", self.id_assets())
    }

    /// URL (même hôte) d'un fichier de la publication de tiles.
    pub fn url_tiles(&self, relatif: &str) -> String {
        format!("{}{relatif}", self.tiles)
    }

    /// URL (même hôte) d'un fichier de la publication d'assets.
    pub fn url_assets(&self, relatif: &str) -> String {
        format!("{}{relatif}", self.assets)
    }
}

/// Lignes utiles d'un MANIFESTE : après `publication` et `version`.
fn entrees(manifeste: &str) -> impl Iterator<Item = &str> {
    manifeste.lines().filter(|l| !l.is_empty() && !l.starts_with("publication ") && !l.starts_with("version "))
}

/// Départements d'une publication de tiles (`D029 <empreinte>` par ligne) ;
/// les autres lignes (`graphe <empreinte>`…) ne sont pas des départements.
pub fn departements(manifeste: &str) -> Vec<String> {
    entrees(manifeste)
        .filter_map(|l| l.split_whitespace().next())
        .filter(|d| est_departement(d))
        .map(str::to_owned)
        .collect()
}

/// Code de département du manifeste : `D` puis trois caractères (`D029`, `D02A`, `D974`).
fn est_departement(d: &str) -> bool {
    d.len() == 4 && d.starts_with('D') && d[1..].bytes().all(|o| o.is_ascii_digit() || o == b'A' || o == b'B')
}

/// Nombre de fichiers d'une publication d'assets (`<sha256>  <chemin>` par ligne).
pub fn fichiers(manifeste: &str) -> usize {
    entrees(manifeste).count()
}

/// Ce que le client sait des données.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EtatDonnees {
    pub publications: Option<Publications>,
    pub departements: Option<Vec<String>>,
    pub fichiers: Option<usize>,
    pub erreur: Option<String>,
}

impl EtatDonnees {
    /// Ligne d'état affichée au joueur.
    pub fn ligne(&self) -> String {
        if let Some(e) = &self.erreur {
            return format!("Données indisponibles : {e}");
        }
        let Some(p) = &self.publications else { return "Données : chargement…".to_owned() };
        let court = |id: &str| id.chars().take(8).collect::<String>();
        let tiles = match &self.departements {
            None => "…".to_owned(),
            Some(d) if d.len() <= 4 => d.join(", "),
            Some(d) => format!("{} départements", d.len()),
        };
        let assets = match self.fichiers {
            None => "…".to_owned(),
            Some(n) => format!("{n} fichiers"),
        };
        format!("Tiles {} : {tiles} — assets {} : {assets}", court(p.id_tiles()), court(p.id_assets()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON: &str = r#"{"tiles":"/tiles/4cf1f60562e68f79/","assets":"/assets/bf566b7e10f4062b/"}"#;

    #[test]
    fn lecture() {
        let p = Publications::lire(JSON).unwrap();
        assert_eq!((p.id_tiles(), p.id_assets()), ("4cf1f60562e68f79", "bf566b7e10f4062b"));
        assert_eq!(p.chemin_asset(POLICE), "bf566b7e10f4062b/polices/DotGothic16-Regular.ttf");
        assert_eq!(p.url_tiles("MANIFESTE"), "/tiles/4cf1f60562e68f79/MANIFESTE");
        assert_eq!(p.url_assets("MANIFESTE"), "/assets/bf566b7e10f4062b/MANIFESTE");
    }

    #[test]
    fn refus() {
        for (a, b) in [
            ("/tiles/4cf1f60562e68f79/", "https://ailleurs.example/tiles/x/"),
            ("/tiles/4cf1f60562e68f79/", "/tiles/../x/"),
            ("/tiles/4cf1f60562e68f79/", "/tiles/4cf1f60562e68f79"),
            ("/tiles/4cf1f60562e68f79/", "/assets/4cf1f60562e68f79/"),
            ("/assets/bf566b7e10f4062b/", "/assets/BF56/"),
            ("/assets/bf566b7e10f4062b/", "/assets/a/b/"),
            ("/assets/bf566b7e10f4062b/", "/assets//"),
        ] {
            assert!(Publications::lire(&JSON.replace(a, b)).is_err(), "accepté : {b}");
        }
        assert!(Publications::lire("pas du json").is_err());
        assert!(Publications::lire(r#"{"tiles":"/tiles/a/"}"#).is_err());
        assert!(Publications::lire(&JSON.replace('}', r#","autre":1}"#)).is_err());
    }

    #[test]
    fn manifestes() {
        let t = "publication 4cf1f60562e68f79\nversion 0.1.0\nD029 eb1c48\nD02A bb\nD974 aa\ngraphe c52bd1a41982396f\n";
        assert_eq!(departements(t), ["D029", "D02A", "D974"]);
        let a = "publication bf56\nversion 0.1.0\n660b  ATTRIBUTIONS.md\n44aa  modeles/arbre.glb\n";
        assert_eq!(fichiers(a), 2);
    }

    #[test]
    fn lignes() {
        let mut e = EtatDonnees::default();
        assert!(e.ligne().contains("chargement"));
        e.publications = Some(Publications::lire(JSON).unwrap());
        assert_eq!(e.ligne(), "Tiles 4cf1f605 : … — assets bf566b7e : …");
        e.departements = Some(vec!["D029".into()]);
        e.fichiers = Some(44);
        assert_eq!(e.ligne(), "Tiles 4cf1f605 : D029 — assets bf566b7e : 44 fichiers");
        e.departements = Some((0..101).map(|i| format!("D{i:03}")).collect());
        assert!(e.ligne().contains("101 départements"));
        e.erreur = Some("HTTP 404".into());
        assert!(e.ligne().starts_with("Données indisponibles"));
    }
}
