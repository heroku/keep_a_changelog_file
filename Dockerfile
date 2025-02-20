FROM rust:latest AS builder

# MUSL is needed to produce a static binary
RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /app
COPY . .

RUN cargo install --target x86_64-unknown-linux-musl --path action

FROM scratch
COPY --from=builder /usr/local/cargo/bin/action /bin/changelog-action
ENTRYPOINT ["/bin/changelog-action"]
