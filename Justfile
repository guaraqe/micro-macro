default:
    @just --list

help:
    @just --list

check:
    cargo check -p micro-macro

check-wasm:
    cargo check -p micro-macro --target wasm32-unknown-unknown

build:
    cargo build -p micro-macro

build-wasm:
    cargo build -p micro-macro --target wasm32-unknown-unknown

build-wasm-release:
    cd crates/micro-macro && trunk build --release --dist web/dist-release --config Trunk.toml

run:
    cargo run -p micro-macro --bin native

serve-wasm:
    cd crates/micro-macro && trunk serve --config Trunk.toml
