FROM rust:1.55-alpine3.13 as base

RUN apk add --no-cache musl-dev

RUN mkdir /src
COPY . /src/

WORKDIR /src
RUN cargo build --release


FROM alpine:3.13

COPY --from=base /src/target/release/genkai_auth_endpoint /usr/local/bin

CMD ["/usr/local/bin/genkai_auth_endpoint"]
