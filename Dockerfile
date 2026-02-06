# =========================
# Stage 1: Build
# =========================
FROM rust:1.75 AS builder

WORKDIR /app

# Copiamos manifest primero para cachear dependencias
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch

# Copiamos el resto del proyecto
COPY src ./src
COPY proto ./proto
COPY build.rs ./

# Build en modo release
RUN cargo build --release

# =========================
# Stage 2: Runtime
# =========================
FROM debian:bookworm-slim

# Dependencias mínimas necesarias
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copiamos el binario compilado
COPY --from=builder /app/target/release/iot_data_saver_service /app/iot_data_saver_service

# Variables por defecto (pueden sobreescribirse)
ENV RUST_LOG=info
ENV ENVIRONMENT=development

EXPOSE 50052

CMD ["./iot_data_saver_service"]

