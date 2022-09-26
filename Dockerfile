FROM rust:1.64 as builder

RUN cargo new --bin openedge
WORKDIR /openedge

COPY ./Cargo.lock ./
COPY ./Cargo.toml ./

RUN cargo build --release
RUN rm src/main.rs && rm ./target/release/openedge

COPY ./src ./src
RUN touch ./src/main.rs
RUN cargo build --release --bin openedge

FROM rust:1.64-slim
COPY --from=builder /openedge/target/release/openedge /bin/openedge
COPY ./hello.js ./
COPY ./goodbye.js ./
CMD [ "/bin/openedge" ]
