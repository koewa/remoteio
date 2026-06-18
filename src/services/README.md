# Services — Protocol & Features

RemoteIO has four internal service modules, each handling a set of WebSocket message types. Messages are dispatched by [`mod.rs`](mod.rs) based on the `type` field.

---

## Sync

**File:** [`sync.rs`](sync.rs)

Manages remote sync targets and executes rsync push/pull operations with dry-run preview.

### Message Types

| Type | Direction | Description |
|---|---|---|
| `sync_list` | Client → Server | Request the list of configured sync targets |
| `sync_save` | Client → Server | Create or update a sync target |
| `sync_delete` | Client → Server | Remove a sync target by name |
| `sync_push` | Client → Server | Start an rsync push (local → remote) |
| `sync_pull` | Client → Server | Start an rsync pull (remote → local) |
| `sync_push_preview` | Client → Server | Dry-run preview for push |
| `sync_pull_preview` | Client → Server | Dry-run preview for pull |
| `sync_list` | Server → Client (broadcast) | Updated target list |
| `sync_status` | Server → Client (broadcast) | Sync operation has started for a target |
| `sync_result` | Server → Client (broadcast) | Sync operation completed (ok/error) with summary |
| `sync_preview` | Server → Client (unicast) | Dry-run result listing affected files |

### Features

- **Target CRUD** — Create, read, update, and delete sync targets with persistent JSON storage (`sync_config.json`)
- **Dual endpoints** — Each target has a local and remote endpoint, each with host, path, and user fields. Empty host = local path
- **rsync integration** — Executes `rsync -avz --delete` for push/pull via `tokio::process::Command`
- **Per-target concurrency** — Each target tracks its sync state in a `HashSet`. The same target cannot run two syncs concurrently; different targets can run in parallel
- **Dry-run preview** — `sync_push_preview` / `sync_pull_preview` runs `rsync --dry-run` and returns the file list. Deleted files are tagged for red-highlighted display
- **Last synced timestamp** — On successful sync, `last_synced` is updated with a human-readable timestamp (e.g. `2026-06-18 12:34:56`)
- **Duplicate button** — Client can duplicate an existing target to pre-fill its form
- **Result notification** — Broadcasts `sync_result` with status (`ok`/`error`), summary, and message to all connected clients
- **Error handling** — Unknown targets return an error message; missing `name` field is silently ignored

### Data Model

```rust
struct SyncTarget {
    name: String,
    local: SyncEndpoint,   // { host, path, user }
    remote: SyncEndpoint,  // { host, path, user }
    last_synced: Option<String>,
}
```

---

## Todo

**File:** [`todo.rs`](todo.rs)

Manages multiple named todo lists with per-item operations and JSON persistence.

### Message Types

| Type | Direction | Description |
|---|---|---|
| `todo_list` | Client → Server | Request all lists and their items |
| `todo_add` | Client → Server | Add a task to a list |
| `todo_remove` | Client → Server | Remove a task by index |
| `todo_reorder` | Client → Server | Move a task from one index to another |
| `todo_edit` | Client → Server | Edit a task's text by index |
| `todo_move` | Client → Server | Move a task from one list to another |
| `todo_list_create` | Client → Server | Create a new named list |
| `todo_list_delete` | Client → Server | Delete a named list |
| `todo_list_rename` | Client → Server | Rename a list |
| `todo_list` | Server → Client (broadcast) | Full list data (all lists and items) |

### Features

- **Multiple named lists** — Tasks are organized into named lists stored in a `HashMap<String, Vec<String>>`. A "default" list always exists
- **Add tasks** — `todo_add` with `text` and optional `list` field (defaults to `"default"`)
- **Remove tasks** — `todo_remove` by index within a list
- **Reorder tasks** — `todo_reorder` with `from`/`to` indices within a list. Also supports drag-and-drop on the client
- **Edit tasks** — `todo_edit` by index with new `text`. Client supports inline double-click editing
- **Move between lists** — `todo_move` transfers a task from one list to another by index and target list name
- **Create/delete/rename lists** — Full list lifecycle. The last list cannot be deleted
- **JSON persistence** — All lists are saved to `todo_store.json` on every mutation
- **Migration support** — Reads old plain-array format and migrates to the new `{ "list_name": [...] }` format
- **Real-time broadcast** — Every mutation broadcasts the full list state to all connected clients via `broadcast_list()`
- **Per-operation list targeting** — Each operation carries an explicit `list` field. The server never stores a "selected list" per client
- **Index safety** — All index-based operations bounds-check before mutating
- **Default list** — If no `list` field is provided, operations default to the `"default"` list

---

## Process

**File:** [`process.rs`](process.rs)

Collects system process information and broadcasts it to all connected clients on a 2-second interval.

### Message Flow

Process data is **not request-driven**. The server automatically broadcasts a JSON array of process objects every 2 seconds to all connected clients via the broadcast channel. The client's WebSocket handler receives it as a top-level JSON array (no `type` field).

### ProcessInfo Fields

| Field | Type | Description |
|---|---|---|
| `pid` | `i32` | Process ID |
| `name` | `String` | Process name (comm) |
| `state` | `String` | Single-character state code (R, S, D, Z, T, etc.) |
| `uid` | `u32` | Real user ID |
| `rss_kb` | `u64` | Resident set size in KB |
| `cpu_time_sec` | `f64` | Total CPU time (user + system) in seconds |
| `cmdline` | `String` | Full command line joined by spaces |

### Features

- **Automatic polling** — `setup_process_monitor()` spawns a background tokio task that polls every 2 seconds
- **Zero-allocation failure** — Returns an empty array if `/proc` is unavailable
- **Page-size aware** — RSS is computed using the system page size
- **CPU time conversion** — Uses `/proc/stat` ticks-per-second to convert jiffies to seconds
- **Real-time broadcast** — Process data is pushed to all connected clients as a JSON array via the broadcast channel
- **Low overhead** — Uses `procfs` crate for efficient `/proc` parsing

### Client Features

- Sortable table with click-to-sort on any column (PID, Name, State, User, RSS, CPU Time, Command)
- Text filter that searches across PID, Name, and UID
- Colored state badges (running=green, sleeping=muted, zombie=red, etc.)
- Human-readable RSS (KB/MB/GB) and CPU time (seconds/minutes/hours)

---

## Server

**File:** [`server.rs`](server.rs)

Handles server-level commands. Currently supports graceful shutdown.

### Message Types

| Type | Direction | Description |
|---|---|---|
| `shutdown` | Client → Server | Initiate graceful server shutdown |

### Features

- **Graceful shutdown** — Sends `"Server shutting down..."` to the requesting socket and broadcasts the same to all connected clients via the channel. Calls `shutdown.notify_one()` on the Axum graceful shutdown signal
- **500ms delay** — Waits 500ms after notifying to give clients time to receive the shutdown message
- **Atomic disconnect** — Returns `true` from `handle_message`, which triggers the WebSocket handler to break out of its receive loop
- **Client confirmation** — The UI shows a confirmation dialog before sending, disables the button during shutdown, and displays "Server offline" once confirmed
