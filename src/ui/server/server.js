import { send } from '../app.js';

const shutdownBtn = document.getElementById("shutdownBtn");

export function onConnect() {
    shutdownBtn.disabled = false;
    shutdownBtn.textContent = "Shutdown Server";
}

export function onDisconnect() {
    shutdownBtn.disabled = true;
    shutdownBtn.textContent = "Server offline";
}

export function onShutdown() {
    shutdownBtn.disabled = true;
    shutdownBtn.textContent = "Server offline";
}

export function init() {
    shutdownBtn.addEventListener("click", function () {
        if (confirm("Are you sure you want to shut down the server?")) {
            send({type: "shutdown"});
            this.disabled = true;
            this.textContent = "Shutting down...";
        }
    });
}
