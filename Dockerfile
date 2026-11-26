# syntax=docker/dockerfile:1
# Client : compilé en wasm32 sur l'image Rust durcie de Docker (dhi.io), servi
# par l'image nginx durcie (non root, port 8080).
FROM dhi.io/rust:1.100.0-debian13-dev@sha256:70b12263cf778cba39a48563b13b55d08ef4c7db5df648d1e1a87c398e1f5f48 AS construction

# Bibliothèque standard wasm32 de Rust 1.100.0 : empreinte relevée sur
# static.rust-lang.org après vérification de la signature GPG de l'archive
# (clé de publication de Rust, 108F 6620 5EAE B0AA A8DD 5E1C 85AB 96E6 FA1B E5FE).
# BuildKit vérifie l'empreinte puis décompresse lui-même (l'image n'a pas gzip).
ADD --unpack=true --checksum=sha256:895cc1555ca67b3a3bcc73814060a4213656a2007c3a46d7c772b3e8467ea066 \
    https://static.rust-lang.org/dist/rust-std-1.100.0-wasm32-unknown-unknown.tar.gz /tmp/
RUN cp -r /tmp/rust-std-1.100.0-wasm32-unknown-unknown/rust-std-wasm32-unknown-unknown/lib/rustlib/wasm32-unknown-unknown \
          "$(rustc --print sysroot)/lib/rustlib/" \
 && rm -rf /tmp/rust-std-1.100.0-wasm32-unknown-unknown

# wasm-bindgen : même version que dans Cargo.lock.
RUN --mount=type=cache,target=/root/.cargo/registry \
    cargo install --locked --root /outils wasm-bindgen-cli@0.2.130

WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    cargo build --release --locked --target wasm32-unknown-unknown \
 && /outils/bin/wasm-bindgen --target web --no-typescript --remove-name-section --out-dir /dist \
      target/wasm32-unknown-unknown/release/beta1_client.wasm \
 && cp web/index.html web/start.js /dist/

FROM dhi.io/nginx:1.31.10-debian13@sha256:bbef6bfb08a559d6ccfa10a37e107204a6c8d21517c06ea07bdf7aefebde0aa8
COPY web/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=construction /dist/ /usr/share/nginx/html/
# Utilisateur « nonroot » de l'image de base, explicite.
USER 65532:65532
EXPOSE 8080
