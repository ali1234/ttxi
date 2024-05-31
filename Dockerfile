FROM rust:alpine as build-env
WORKDIR /ttxi
COPY . .
RUN cargo build --release

FROM alpine
WORKDIR /ttxi
COPY --from=build-env /ttxi/target/release/ttxi /ttxi/ttxi
