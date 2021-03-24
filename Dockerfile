FROM rustlang/rust:nightly-alpine3.12 as build

COPY src /build/src
COPY Cargo.toml /build/

WORKDIR /build
RUN apk add --no-cache musl-dev && \
    cargo build --release

FROM scratch
COPY --from=build /build/target/release/mega_mailer /opt/mega-mailer/mega_mailer
COPY config.yaml /opt/mega-mailer
