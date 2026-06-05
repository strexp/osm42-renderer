# osm42-renderer

A raster tile server that renders vector tiles to PNG format using `maplibre-native`.

## Building

```bash
cargo build --release
```

## Usage

Start the server with default parameters:

```bash
./target/release/osm42-renderer
```

### API Endpoint

- `GET /catalog`
  Returns a standard JSON array of available map styles synced from the upstream layer.

- `GET /style/{style}`
  Returns a Mapbox/MapLibre compatible style JSON configured with a raster source pointing to this server's tile generation endpoint.

- `GET /{style}/{z}/{x}/{y}`
  Fetches the tile for the specified style and XYZ coordinates, returning an `image/png` response.

## Configuration

The server can be configured via command-line arguments or environment variables. You can also use a `.env` file.

| Argument | Environment Variable | Default Value | Description |
| :--- | :--- | :--- | :--- |
| `--bind-ip` | `BIND_IP` | `[::]` | IP address to bind to. |
| `--port` | `PORT` | `8080` | Port to listen on. |
| `--style-base-url` | `STYLE_BASE_URL` | `http://localhost:8080` | Base URL used to resolve map styles. |
| `--cache-dir` | `CACHE_DIR` | `./tiles_cache` | Directory path for file system cache. |
| `--cache-ttl` | `CACHE_TTL_SECONDS` | `86400` | File cache time-to-live in seconds. |
| `--disable-cache` | `DISABLE_CACHE` | `false` | Disables both memory and file caching. |
| `--render-worker-queue-size`| `RENDER_WORKER_QUEUE_SIZE` | `1024` | Maximum job queue capacity per render worker. |
| `--render-num-workers` | `RENDER_NUM_WORKERS` | *CPU cores* | Number of maplibre rendering threads. |
| `--render-stack-size-mb` | `RENDER_STACK_SIZE_MB` | `8` | Stack size for render threads in megabytes. |

## Benchmark Mode

The application includes a built-in benchmark mode to test rendering performance. When run in benchmark mode, the server does not start the HTTP listener.

```bash
./target/release/tile_server --benchmark [OPTIONS]
```

### Benchmark Configuration

| Argument | Environment Variable | Default Value | Description |
| :--- | :--- | :--- | :--- |
| `--benchmark` | N/A | `false` | Activates benchmark mode. |
| `--benchmark-concurrency` | `BENCHMARK_CONCURRENCY` | `500` | Number of concurrent render requests. |
| `--benchmark-duration-secs` | `BENCHMARK_DURATION_SECS` | `10` | Benchmark duration in seconds. |

Example:

```bash
./target/release/tile_server --benchmark --benchmark-concurrency 200 --benchmark-duration-secs 30
```

## License

AGPL-3.0
