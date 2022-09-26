FROM rust:1.64 as builder

RUN cargo new --bin deno-edge
WORKDIR /deno-edge

COPY ./Cargo.lock ./
COPY ./Cargo.toml ./

RUN cargo build --release
RUN rm src/main.rs && rm ./target/release/deno-edge

COPY ./src ./src
RUN touch ./src/main.rs
RUN cargo build --release --bin deno-edge

FROM rust:1.64-slim
COPY --from=builder /deno-edge/target/release/deno-edge /bin/deno-edge
COPY ./hello.js ./
COPY ./goodbye.js ./
CMD [ "/bin/deno-edge" ]
