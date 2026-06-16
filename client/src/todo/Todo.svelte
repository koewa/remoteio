<script>
  import { onMount } from 'svelte';
  import { todos, send } from '../lib/ws.js';

  let newText = '';
  let dragFrom = null;

  onMount(() => {
    send({ type: 'todo_list' });
  });

  function esc(s) {
    const d = document.createElement('div');
    d.textContent = s;
    return d.innerHTML;
  }

  function add() {
    const text = newText.trim();
    if (text) {
      send({ type: 'todo_add', text });
      newText = '';
    }
  }

  function handleKeydown(e) {
    if (e.key === 'Enter') add();
  }

  function remove(id) {
    send({ type: 'todo_remove', id });
  }

  function dragstart(e, id) {
    dragFrom = id;
    e.dataTransfer.setData('text/plain', id);
    e.target.classList.add('dragging');
  }

  function dragend(e) {
    e.target.classList.remove('dragging');
    document.querySelectorAll('.drag-over').forEach(el => el.classList.remove('drag-over'));
  }

  function dragover(e) {
    e.preventDefault();
  }

  function dragenter(e) {
    e.preventDefault();
    e.currentTarget.classList.add('drag-over');
  }

  function dragleave(e) {
    e.currentTarget.classList.remove('drag-over');
  }

  function drop(e, to) {
    e.preventDefault();
    e.currentTarget.classList.remove('drag-over');
    if (dragFrom !== null && dragFrom !== to) {
      send({ type: 'todo_reorder', from: dragFrom, to });
    }
    dragFrom = null;
  }

  function startEdit(li, id) {
    const original = $todos[id];
    const span = li.querySelector('.todo-text');
    const input = document.createElement('input');
    input.type = 'text';
    input.value = original;
    input.maxLength = 200;
    input.className = 'todo-edit-input';
    span.replaceWith(input);
    input.focus();
    input.select();

    function finish() {
      const text = input.value.trim();
      if (text && text !== original) {
        send({ type: 'todo_edit', id, text });
      }
    }

    input.addEventListener('blur', finish);
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') input.blur();
      if (e.key === 'Escape') input.value = original;
    });
  }
</script>

<div class="todo-input">
  <input type="text" id="todoInput" placeholder="Add a task..." maxlength="200"
    bind:value={newText} onkeydown={handleKeydown}>
  <button id="todoAddBtn" onclick={add}>Add</button>
</div>

<ul id="todoList" class="todo-list">
  {#if $todos.length === 0}
    <li class="todo-empty">No tasks yet. Add one above.</li>
  {:else}
    {#each $todos as item, i}
      <li class="todo-item" draggable="true"
        ondragstart={(e) => dragstart(e, i)}
        ondragend={dragend}
        ondragover={dragover}
        ondragenter={dragenter}
        ondragleave={dragleave}
        ondrop={(e) => drop(e, i)}>
        <span class="todo-text" role="button" tabindex="-1" ondblclick={(e) => startEdit(e.currentTarget.closest('li'), i)}>
          {esc(item)}
        </span>
        <button class="todo-remove" onclick={() => remove(i)}>✕</button>
      </li>
    {/each}
  {/if}
</ul>

<style>
  .todo-input { display: flex; gap: 8px; margin-bottom: 16px; }
  .todo-input input {
    flex: 1; max-width: 400px; padding: 8px 12px;
    background: #161b22; border: 1px solid #30363d; border-radius: 6px;
    color: #c9d1d9; font-size: 0.9rem; outline: none;
  }
  .todo-input input:focus { border-color: #58a6ff; }
  .todo-input button {
    padding: 8px 18px; font-size: 0.85rem; font-weight: 600;
    background: #238636; color: #fff; border: none; border-radius: 6px; cursor: pointer;
  }
  .todo-input button:hover { background: #2ea043; }

  .todo-list { list-style: none; max-width: 500px; }
  .todo-item {
    display: flex; align-items: center; gap: 10px;
    padding: 10px 12px; border-bottom: 1px solid #21262d;
  }
  .todo-item:hover { background: #161b22; border-radius: 6px; }
  .todo-text { flex: 1; font-size: 0.9rem; cursor: default; }
  .todo-remove {
    padding: 2px 8px; font-size: 0.8rem;
    background: transparent; color: #da3633; border: 1px solid #da3633;
    border-radius: 4px; cursor: pointer; font-weight: 600;
  }
  .todo-remove:hover { background: #da3633; color: #fff; }
  .todo-empty { color: #8b949e; font-size: 0.9rem; padding: 20px 0; }

  :global(.dragging) { opacity: 0.4; }
  :global(.drag-over) { border-top: 2px solid #58a6ff; }

  :global(.todo-edit-input) {
    flex: 1; padding: 4px 8px; font-size: 0.9rem; font-family: inherit;
    background: #0d1117; border: 1px solid #58a6ff; border-radius: 4px;
    color: #c9d1d9; outline: none;
  }
</style>
