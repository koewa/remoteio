import { writable } from 'svelte/store';

export const processes = writable([]);
export const todos = writable([]);
export const connected = writable(false);
export const shuttingDown = writable(false);
export const syncStates = writable({});
export const syncTargets = writable([]);

let socket;
let pending = [];

export function send(msg) {
  if (socket && socket.readyState === WebSocket.OPEN) {
    socket.send(JSON.stringify(msg));
  } else {
    pending.push(msg);
  }
}

function flushPending() {
  for (const msg of pending) {
    socket.send(JSON.stringify(msg));
  }
  pending = [];
}

export function connect() {
  socket = new WebSocket('ws://' + window.location.host + '/ws');

  socket.onopen = () => {
    connected.set(true);
    flushPending();
    send({ type: 'todo_list' });
    send({ type: 'sync_list' });
  };

  socket.onmessage = (event) => {
    const msg = event.data;
    if (msg === 'Server shutting down...') {
      shuttingDown.set(true);
      return;
    }
    try {
      const data = JSON.parse(msg);
      if (Array.isArray(data)) {
        processes.set(data);
      } else if (data.type === 'todo_list') {
        todos.set(data.items || []);
      } else if (data.type === 'sync_list') {
        syncTargets.set(data.targets || []);
      } else if (data.type === 'sync_status') {
        syncStates.update(states => ({
          ...states,
          [data.name]: { syncing: true, direction: data.direction, lastResult: null }
        }));
      } else if (data.type === 'sync_result') {
        syncStates.update(states => ({
          ...states,
          [data.name]: { syncing: false, direction: data.direction, lastResult: { status: data.status, summary: data.summary || data.message, timestamp: new Date().toLocaleString() } }
        }));
      }
    } catch (_) {}
  };

  socket.onclose = () => {
    connected.set(false);
    shuttingDown.set(false);
    processes.set([]);
    setTimeout(connect, 3000);
  };

  socket.onerror = () => {
    connected.set(false);
  };
}
