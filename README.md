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

## docker

1. Image bauen:
```bash
docker build -t rusterizer .
```

2. Container starten:
```bash
docker run -d -p 4757:8000 --init --restart always rusterizer
```
*publish port directly to network interface*
```bash
docker run -d --init --network=host --restart always rusterizer
```