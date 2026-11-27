# beta1-client

Client navigateur de la beta 1 de **La Bataille de France** : Bevy compilé en WebAssembly, rendu en **WebGPU seul**. Il affiche l'état reçu du serveur ([beta1-server](https://github.com/labatailledefrance/beta1-server)) et envoie des ordres ; il ne simule rien. Code partagé : [beta1-common](https://github.com/labatailledefrance/beta1-common).

## Construire et lancer

```
rustup target add wasm32-unknown-unknown
cargo install --locked wasm-bindgen-cli@0.2.130     # même version que Cargo.lock
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --remove-name-section --out-dir dist \
  target/wasm32-unknown-unknown/release/beta1_client.wasm
cp web/index.html web/start.js dist/
python3 -m http.server -d dist 8000
```

Puis ouvrir `http://127.0.0.1:8000/?server=ws://127.0.0.1:7425` (serveur local). Sans paramètre, le client se connecte à `/ws` sur le même hôte.

## Données

Au démarrage, le client lit `/publications.json` sur le même hôte. Ce fichier est servi par le serveur de données du chart `game` ([H-06](https://github.com/labatailledefrance/documentation/blob/main/OPEN-PROBLEMS.md#h-06--diffusion-des-tiles)) et donne les chemins des publications courantes, `/tiles/<id>/` et `/assets/<id>/`. Leur contenu est immuable, donc gardé en cache sans expiration. Le client refuse tout autre chemin ou hôte. Il lit ensuite les deux `MANIFESTE` (départements, nombre de fichiers) et charge la police de l'interface, DotGothic16, depuis la publication d'assets. La racine web de l'AssetServer de Bevy est `assets/`, si bien que `assets.load("<id>/…")` lit `/assets/<id>/…`.

En local, sans serveur de données, la ligne des données affiche l'erreur et le reste fonctionne.

En natif, seule la logique (`src/state.rs`) se compile et se teste : `cargo test`.

## Image

Construite sur `dhi.io/rust` (bibliothèque standard wasm32 téléchargée par empreinte, signature GPG vérifiée), servie par `dhi.io/nginx` (non root, port 8080, en-têtes COOP, COEP et CSP, compression gzip). Publiée sur ghcr.io avec SBOM et provenance, puis signée (cosign).

Contribution : [CONTRIBUTING.md](https://github.com/labatailledefrance/.github/blob/main/CONTRIBUTING.md). Licence : AGPL-3.0-or-later.
