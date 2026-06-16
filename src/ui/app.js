import * as processService from './process/process.js';
import * as todoService from './todo/todo.js';
import * as serverService from './server/server.js';

const statusEl = document.getElementById("status");
const connectingEl = document.getElementById("connecting");

let socket;

export function send(msg) {
    if (socket && socket.readyState === WebSocket.OPEN) {
        socket.send(JSON.stringify(msg));
    }
}

export function esc(s) {
    const d = document.createElement("div");
    d.textContent = s;
    return d.innerHTML;
}

function connect() {
    socket = new WebSocket("ws://" + window.location.host + "/ws");

    socket.onopen = () => {
        statusEl.textContent = "status: connected";
        connectingEl.style.display = "none";
        serverService.onConnect();
        send({type: "todo_list"});
    };

    socket.onmessage = (event) => {
        const msg = event.data;
        if (msg === "Server shutting down...") {
            serverService.onShutdown();
            return;
        }
        try {
            const data = JSON.parse(msg);
            if (Array.isArray(data)) {
                processService.onMessage(data);
            } else if (data.type === "todo_list") {
                todoService.onMessage(data);
            }
        } catch (_) {}
    };

    socket.onclose = () => {
        statusEl.textContent = "status: disconnected";
        connectingEl.textContent = "Reconnecting in 3 seconds...";
        connectingEl.style.display = "block";
        serverService.onDisconnect();
        processService.onDisconnect();
        setTimeout(connect, 3000);
    };

    socket.onerror = () => {
        statusEl.textContent = "status: error";
    };
}

processService.init();
todoService.init();
serverService.init();
connect();
