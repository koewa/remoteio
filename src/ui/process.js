import { esc } from './app.js';

const tbody = document.getElementById("tbody");
const countEl = document.getElementById("count");
const filterInput = document.getElementById("filter");

let processes = [];
let sortCol = "pid";
let sortDir = 1;

function fmtBytes(kb) {
    if (kb >= 1048576) return (kb / 1048576).toFixed(1) + " GB";
    if (kb >= 1024) return (kb / 1024).toFixed(1) + " MB";
    return kb + " KB";
}

function fmtCpu(sec) {
    if (sec >= 3600) return (sec / 3600).toFixed(1) + "h";
    if (sec >= 60) return (sec / 60).toFixed(1) + "m";
    return sec.toFixed(1) + "s";
}

function stateClass(s) {
    const cls = { "R": "state-R", "S": "state-S", "D": "state-D",
                  "Z": "state-Z", "T": "state-T" };
    return cls[s] || "";
}

function stateLabel(s) {
    const map = { "R": "running", "S": "sleeping", "D": "disk sleep",
                  "Z": "zombie", "T": "stopped" };
    return map[s] || s;
}

function render() {
    const q = filterInput.value.toLowerCase();
    const filtered = processes.filter(p =>
        p.pid.toString().includes(q) ||
        p.name.toLowerCase().includes(q) ||
        p.uid.toString().includes(q)
    );

    const sorted = [...filtered].sort((a, b) => {
        let va = a[sortCol], vb = b[sortCol];
        if (typeof va === "string") va = va.toLowerCase();
        if (typeof vb === "string") vb = vb.toLowerCase();
        if (va < vb) return -sortDir;
        if (va > vb) return sortDir;
        return 0;
    });

    let html = "";
    for (const p of sorted) {
        html += `<tr>
            <td class="pid">${esc(p.pid)}</td>
            <td>${esc(p.name)}</td>
            <td><span class="state ${stateClass(p.state)}">${stateLabel(p.state)}</span></td>
            <td>${p.uid}</td>
            <td class="rss">${fmtBytes(p.rss_kb)}</td>
            <td class="cpu">${fmtCpu(p.cpu_time_sec)}</td>
            <td class="cmdline" title="${esc(p.cmdline)}">${esc(p.cmdline)}</td>
        </tr>`;
    }
    tbody.innerHTML = html;
    countEl.textContent = `processes: ${processes.length}`;
}

export function onMessage(data) {
    processes = data;
    render();
}

export function onDisconnect() {
    processes = [];
    render();
}

export function init() {
    document.querySelectorAll("th[data-col]").forEach(th => {
        th.addEventListener("click", () => {
            const col = th.dataset.col;
            if (sortCol === col) sortDir *= -1;
            else { sortCol = col; sortDir = 1; }
            document.querySelectorAll("th.sorted").forEach(t => t.classList.remove("sorted"));
            th.classList.add("sorted");
            th.textContent = th.textContent.replace(/ [▴▾]/, "");
            th.textContent += sortDir === 1 ? " ▴" : " ▾";
            render();
        });
    });

    filterInput.addEventListener("input", render);
}
