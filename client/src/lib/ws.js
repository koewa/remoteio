import { writable } from 'svelte/store';

export const processes = writable([]);
export const todos = writable([]);
export const connected = writable(false);
export const shuttingDown = writable(false);
export const syncState = writable({ syncing: false, direction: null, lastResult: null });

let socket;

export function send(msg) {
  if (socket && socket.readyState === WebSocket.OPEN) {
    socket.send(JSON.stringify(msg));
  }
}

export function connect() {
  socket = new WebSocket('ws://' + window.location.host + '/ws');

  socket.onopen = () => {
    connected.set(true);
    send({ type: 'todo_list' });
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
      } else if (data.type === 'sync_status') {
        syncState.set({ syncing: true, direction: data.direction, lastResult: null });
      } else if (data.type === 'sync_result') {
        syncState.set({ syncing: false, direction: null, lastResult: { direction: data.direction, status: data.status, summary: data.summary || data.message, timestamp: new Date().toLocaleString() } });
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
