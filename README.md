# RemoteIO

RemoteIO is a browser-based server management tool with a real-time WebSocket interface. It provides sync management, process monitoring, task tracking, and server control — all from a web UI.

## Features

- **Sync Targets** — Configure push/pull rsync targets with dry-run preview
- **Process Viewer** — Real-time system process table with filtering and sorting
- **Todo Lists** — Multiple named task lists with inline editing and drag reorder
- **Server Control** — Graceful server shutdown from the browser

## Quick Start

```bash
# Run the server
cargo run

# Open http://127.0.0.1:8080
```

For a custom address:
```bash
cargo run -- --ip 0.0.0.0 --port 3000
```

## Development

### Prerequisites

- Rust 1.75+
- Node.js 20+ (for client development)
- rsync (for sync features)

### Build

```bash
# Server
cargo build

# Client
cd client && npm install && npm run build
```

### Run Tests

```bash
cargo test
```

## Project Structure

```
├── src/
│   ├── main.rs              # Entry point, WebSocket handler, integration tests
│   └── services/
│       ├── mod.rs            # Message dispatch routing
│       ├── types.rs          # Shared Status struct and types
│       ├── sync.rs           # Remote sync (rsync) management
│       ├── todo.rs           # Todo list management
│       ├── process.rs        # System process monitoring
│       └── server.rs         # Server shutdown command
├── client/
│   ├── src/
│   │   ├── App.svelte        # Root tab shell
│   │   ├── lib/ws.js         # WebSocket stores and communication
│   │   ├── sync/Sync.svelte  # Sync targets UI
│   │   ├── todo/Todo.svelte  # Todo list UI
│   │   ├── process/Process.svelte  # Process viewer UI
│   │   └── server/Server.svelte    # Shutdown button UI
│   └── dist/                 # Built client assets (served statically)
```

## Architecture

The server communicates with browser clients exclusively over a WebSocket at `/ws`. All state mutations happen on the server; the UI is a thin client that renders real-time updates via broadcast messages.

```
Browser ──WebSocket──> Server (Axum)
                            │
                    ┌───────┼───────────┐
                    │       │           │
                 sync    todo      process
                 .rs     .rs        .rs
```

### Message Format

All messages are JSON with a `type` field. See the service READMEs for detailed protocol documentation.

## Services

Each service is documented individually:

| Service | File | Description |
|---|---|---|
| Sync | [`src/services/sync.rs`](src/services/README.md#sync) | rsync push/pull with target management, dry-run preview, per-target concurrency |
| Todo | [`src/services/todo.rs`](src/services/README.md#todo) | Multiple named task lists with add/remove/reorder/edit/move, JSON persistence |
| Process | [`src/services/process.rs`](src/services/README.md#process) | Real-time system process collection with 2-second polling interval |
| Server | [`src/services/server.rs`](src/services/README.md#server) | Graceful server shutdown |

## License

MIT
