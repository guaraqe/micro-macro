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

run:
    cargo run -p micro-macro --bin native

serve-wasm:
    cd crates/micro-macro && trunk serve --config Trunk.toml
