
# ---- build stage ----
FROM rust:1.85 as builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo "fn main(){}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

COPY . .
RUN touch src/main.rs && cargo build --release

# ---- runtime stage ----
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/bff /usr/local/bin/bff
EXPOSE 8081
ENV BIND_ADDR=0.0.0.0:8081
CMD ["bff"]

