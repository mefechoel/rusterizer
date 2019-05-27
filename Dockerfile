FROM rust:1.35.0-slim

WORKDIR /usr/src/rusterizer
COPY . .

RUN rustup install nightly-2019-04-18 && \
    rustup default nightly-2019-04-18 && \
    cargo build --release

EXPOSE 8000

CMD ["cargo", "run", "--release"]