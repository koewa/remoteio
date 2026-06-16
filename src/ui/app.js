import * as processService from './process/process.js';
import * as todoService from './todo/todo.js';
import * as serverService from './server/server.js';

const statusEl = document.getElementById("status");

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

async function loadPanel(id, url) {
    const resp = await fetch(url);
    document.getElementById(id).innerHTML = await resp.text();
}

function connect() {
    socket = new WebSocket("ws://" + window.location.host + "/ws");

    socket.onopen = () => {
        statusEl.textContent = "status: connected";
        serverService.onConnect();
        processService.onConnect();
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
        serverService.onDisconnect();
        processService.onDisconnect();
        setTimeout(connect, 3000);
    };

    socket.onerror = () => {
        statusEl.textContent = "status: error";
    };
}

async function init() {
    await Promise.all([
        loadPanel("panel-processes", "/process/process.html"),
        loadPanel("panel-todo", "/todo/todo.html"),
    ]);

    document.querySelectorAll(".tab").forEach(tab => {
        tab.addEventListener("click", () => {
            document.querySelectorAll(".tab").forEach(t => t.classList.remove("active"));
            document.querySelectorAll(".panel").forEach(p => p.classList.remove("active"));
            tab.classList.add("active");
            document.getElementById("panel-" + tab.dataset.tab).classList.add("active");
        });
    });

    processService.init();
    todoService.init();
    serverService.init();
    connect();
}

init();
