import { esc, send } from './app.js';

const todoInput = document.getElementById("todoInput");
const todoAddBtn = document.getElementById("todoAddBtn");
const todoList = document.getElementById("todoList");

let todos = [];

function render() {
    if (todos.length === 0) {
        todoList.innerHTML = '<li class="todo-empty">No tasks yet. Add one above.</li>';
        return;
    }
    let html = "";
    for (let i = 0; i < todos.length; i++) {
        html += `<li class="todo-item" draggable="true" data-id="${i}">
            <span class="todo-text">${esc(todos[i])}</span>
            <button class="todo-remove" data-id="${i}">✕</button>
        </li>`;
    }
    todoList.innerHTML = html;
    todoList.querySelectorAll(".todo-remove").forEach(btn => {
        btn.addEventListener("click", () => {
            const id = parseInt(btn.dataset.id);
            send({type: "todo_remove", id});
            send({type: "todo_list"});
        });
    });
            todoList.querySelectorAll(".todo-text").forEach(span => {
                const li = span.closest(".todo-item");
                const id = parseInt(li.dataset.id);
                span.addEventListener("dblclick", () => {
                    const original = todos[id];
                    const input = document.createElement("input");
                    input.type = "text";
                    input.value = original;
                    input.maxLength = 200;
                    input.className = "todo-edit-input";
                    span.replaceWith(input);
                    input.focus();
                    input.select();

                    function finish() {
                        const text = input.value.trim();
                        if (text && text !== original) {
                            send({type: "todo_edit", id, text});
                        }
                        send({type: "todo_list"});
                    }

                    input.addEventListener("blur", finish);
                    input.addEventListener("keydown", (e) => {
                        if (e.key === "Enter") input.blur();
                        if (e.key === "Escape") send({type: "todo_list"});
                    });
                });
            });
            todoList.querySelectorAll(".todo-item").forEach(item => {
        item.addEventListener("dragstart", (e) => {
            e.dataTransfer.setData("text/plain", item.dataset.id);
            item.classList.add("dragging");
        });
        item.addEventListener("dragend", () => {
            item.classList.remove("dragging");
            document.querySelectorAll(".todo-item.drag-over").forEach(el => el.classList.remove("drag-over"));
        });
        item.addEventListener("dragover", (e) => e.preventDefault());
        item.addEventListener("dragenter", (e) => {
            e.preventDefault();
            item.classList.add("drag-over");
        });
        item.addEventListener("dragleave", () => {
            item.classList.remove("drag-over");
        });
        item.addEventListener("drop", (e) => {
            e.preventDefault();
            item.classList.remove("drag-over");
            const from = parseInt(e.dataTransfer.getData("text/plain"));
            const to = parseInt(item.dataset.id);
            if (from !== to) {
                send({type: "todo_reorder", from, to});
                send({type: "todo_list"});
            }
        });
    });
}

export function onMessage(data) {
    todos = data.items || [];
    render();
}

export function init() {
    todoAddBtn.addEventListener("click", () => {
        const text = todoInput.value.trim();
        if (text) {
            send({type: "todo_add", text});
            todoInput.value = "";
            todoInput.focus();
            send({type: "todo_list"});
        }
    });

    todoInput.addEventListener("keydown", (e) => {
        if (e.key === "Enter") todoAddBtn.click();
    });
}
