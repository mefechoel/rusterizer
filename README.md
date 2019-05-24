# rusterizer

## setup
Rustup [herunterladen](https://rustup.rs/) und installieren.

Dann richtige toolchain einstellen:
```bash
rustup install nightly-2019-04-18
rustup default nightly-2019-04-18
```

## starten

Code kompilieren und ausführen:
```bash
cargo run
```

## production

Unbedingt in release mode bauen!
```bash
cargo build --release
cargo run --release
```
