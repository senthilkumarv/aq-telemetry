# Aquarium Telemetry Service

A high-performance telemetry service for aquarium monitoring systems, built with Rust and using Apache Thrift for efficient data serialization.

## Features

- **Real-time Data**: Queries InfluxDB for live aquarium telemetry data
- **Multi-Aquarium Support**: Supports multiple aquariums in a single deployment
- **Efficient Protocol**: Uses Apache Thrift binary protocol for compact data representation
- **Brotli Compression**: Responses are compressed with Brotli for minimal bandwidth usage
- **Configuration-Driven**: Widget and query definitions are loaded from TOML files for easy updates without code changes
- **IDL-Based**: Uses published IDL library for type-safe cross-platform communication

## Architecture

This service follows **Domain-Driven Design (DDD)** principles with a clean, layered architecture. See [ARCHITECTURE.md](./ARCHITECTURE.md) for detailed architecture documentation.

### Technology Stack

- **Language**: Rust (Edition 2024)
- **Web Framework**: Axum
- **Serialization**: Apache Thrift (Binary Protocol)
- **Compression**: Brotli
- **Database**: InfluxDB (v1 QL compatibility mode)
- **Configuration**: TOML

### IDL Library

The service uses the published `telemetry-thrift` library from:
- Repository: https://github.com/senthilkumarv/aq-telemetry-idl.git
- Tag: v0.0.1

## API Endpoints

### GET /healthz

Health check endpoint.

**Response**: Plain text "ok"

### GET /aquariums

Returns a list of all available aquariums.

**Response**: Thrift-encoded list of `SDAquarium` objects, compressed with Brotli
- Content-Type: `application/x-thrift`
- Content-Encoding: `br`

### GET /dashboards/:id?hours=N

Returns dashboard data for a specific aquarium.

**Parameters**:
- `id`: Aquarium identifier (e.g., "Great_Barrier_", "Planet_72")
- `hours`: Time range in hours (default: 6)

**Response**: Thrift-encoded `SDPage` object, compressed with Brotli
- Content-Type: `application/x-thrift`
- Content-Encoding: `br`

## Configuration

### InfluxDB Configuration

Located at `config/influx.toml`:

```toml
[influx]
host = "https://us-east-1-1.aws.cloud2.influxdata.com"
token = "..."
database = "neptune"
retention_policy = "autogen"
```

### Widget Configuration

Located at `config/widgets.toml`:

Defines tiles (single-value metrics) and charts (time-series data) with their associated InfluxQL queries.

**Template Variables**:
- `${source}`: Replaced with aquarium ID
- `${hours}`: Replaced with time range in hours

Example tile:
```toml
[[tiles]]
id = "t-temp"
title = "Temperature"
unit = "°F"
precision = 1
query = "SELECT MEAN(value) FROM \"apex_probe\" WHERE \"host\"='${source}' AND \"probe_type\"='temp' AND time >= now() - 2m"
```

Example chart:
```toml
[[charts]]
id = "c-temp"
title = "Temperature"
unit = "°F"
kind = "multiLine"
y_min = 76.0
y_max = 80.0
fraction_digits = 1
  [[charts.series]]
  id = "s-temp-base"
  name = "Tmp"
  color = "#007aff"
  query = "SELECT LAST(value) AS value FROM \"apex_probe\" WHERE \"host\"='${source}' AND \"probe_type\"='temp' AND \"name\"='Tmp' AND value > 50 AND time >= now() - ${hours}h GROUP BY time(1m) fill(none)"
```

## Building and Running

### Prerequisites

- Rust (nightly toolchain)
- Cargo

### Build

```bash
cd aquarium-telemetry
cargo build --release
```

### Run

```bash
cd aquarium-telemetry
cargo run
```

The service will start on `0.0.0.0:8080`.

## Testing

Run the test script:

```bash
# Test health check
curl http://localhost:8080/healthz

# Test aquariums list
curl -v http://localhost:8080/aquariums

# Test dashboard
curl -v "http://localhost:8080/dashboards/Great_Barrier_?hours=6"
```

## Data Model

The service uses the following Thrift types from the IDL:

- `SDAquarium`: Aquarium metadata (id, name)
- `SDPage`: Dashboard page (title, tiles, charts, overlays)
- `SDTile`: Single-value metric (id, title, unit, value, precision)
- `SDChart`: Time-series chart (id, title, unit, kind, y_min, y_max, series)
- `SDSeries`: Data series (id, name, color, points)
- `SDPoint`: Time-series data point (timestamp_ms, value)
- `ChartKind`: Enum (LINE, MULTILINE)

## Performance

- Thrift binary protocol provides compact serialization
- Brotli compression reduces response sizes by ~70-80%
- Typical response sizes:
  - `/aquariums`: ~95 bytes (compressed)
  - `/dashboards/:id`: ~1-2 KB (compressed, 1-6 hours of data)

