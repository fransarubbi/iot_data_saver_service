# IoT Data Saver Service

A high-performance Rust backend service for aggregating IoT sensor data and operational metrics from edge devices. Receives data via gRPC bidirectional streams, batches them efficiently, persists to PostgreSQL, and sends periodic heartbeats for health monitoring.

## 🎯 Purpose

This service acts as a bridge between IoT edge devices and a data warehouse. Instead of storing individual readings (inefficient), it collects them in memory batches and inserts them in bulk, achieving high throughput with minimal database load.

**Key Use Cases:**
- Collecting environmental sensor readings (temperature, humidity, CO2)
- Monitoring system health metrics (CPU, RAM, disk, network)
- Capturing alert events in real-time
- Ensuring edge API connectivity via heartbeat signals

---

## ✨ Features

| Feature | Description |
|---------|-------------|
| **gRPC Bidirectional Streaming** | Efficient binary protocol for continuous data flow |
| **Batching System** | Accumulates messages, inserts in configurable batches |
| **Type-Safe Queries** | SQLx compile-time validation prevents SQL bugs |
| **Actor Model** | Message-passing concurrency with Tokio async runtime |
| **Resilient Connections** | Auto-reconnect with exponential backoff for gRPC and DB |
| **Smart Logging** | Pretty logs for dev, JSON for production cloud platforms |
| **Centralized Configuration** | Single `.env` file controls all behavior |
| **Heartbeat Monitoring** | Periodic keep-alive signals to edge API |

---

## 📋 Prerequisites

- **Rust** 1.75+ ([install](https://rustup.rs/))
- **PostgreSQL** 14+ (local or remote)
- **Docker** (optional, recommended)
- **Git**

---

## 🚀 Quick Start

### Option 1: Local Development

```bash
# 1. Clone repository
git clone https://github.com/fransarubbi/iot_data_saver_service.git
cd iot_data_saver_service

# 2. Setup environment
cp .env.example .env
# Edit .env with your PostgreSQL credentials

# 3. Create database schema
createdb iot_data
psql iot_data < init_db.sql

# 4. Build and run
cargo build --release
RUST_LOG=info cargo run --release
```

### Option 2: Docker Compose (Recommended)

```bash
This option runs the full local stack using Docker:
- PostgreSQL
- Data Saver gRPC service

```bash
# 1. Clone repository
git clone https://github.com/fransarubbi/iot_data_saver_service.git
cd iot_data_saver_service

# 2. Build and start stack
docker compose up --build

# 3. Follow service logs
docker compose logs -f iot_data_saver
````

---

## ⚙️ Configuration

The service is fully configured via environment variables.
When using Docker Compose, these are defined directly in `docker-compose.yml`.

### Core Settings

```bash
# Database connection
DATABASE_URL=postgresql://iot_user:iot_password@postgres:5432/iot_data

# gRPC server configuration
GRPC_HOST=0.0.0.0
GRPC_PORT=50052

# Heartbeat interval
HEARTBEAT_INTERVAL_SECS=30

# Logging
RUST_LOG=info                    # trace, debug, info, warn, error
ENVIRONMENT=development          # development, staging, production

# Database pool
DB_POOL_SIZE=10
```

### Environment Profiles

#### Development

```bash
ENVIRONMENT=development
RUST_LOG=debug
```

→ Pretty-printed, colorful logs

#### Production

```bash
ENVIRONMENT=production
RUST_LOG=warn
```

→ JSON logs for log aggregation systems (ELK, Datadog, etc)

---

## 🔍 Logging & Observability

### Log Levels

- **DEBUG:** Detailed internal state (timestamps, channel activity)
- **INFO:** Normal operations (startup, received messages)
- **WARN:** Recoverable issues (reconnect attempts)
- **ERROR:** Critical failures (DB unavailable, gRPC crash)

### Production JSON Logs

In production, logs are structured JSON:
```json
{
  "timestamp": "2026-02-06T18:30:45.123Z",
  "level": "INFO",
  "message": "Heartbeat sent",
  "target": "iot_data_saver_service::heartbeat",
  "module_path": "iot_data_saver_service::heartbeat::logic"
}
```

Easily ingested by: **Elasticsearch**, **Datadog**, **CloudWatch**, **Splunk**, etc.
