FROM rust:1.35.0

WORKDIR /usr/src/rusterizer
COPY . .

RUN rustup install nightly-2019-04-18 && \
    rustup default nightly-2019-04-18 && \
    cargo build --release

EXPOSE 4757

CMD ["cargo", "run", "--release"]