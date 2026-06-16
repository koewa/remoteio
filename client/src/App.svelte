<script>
  import { onMount } from 'svelte';
  import { connect, connected, processes } from './lib/ws.js';
  import Server from './server/Server.svelte';
  import Process from './process/Process.svelte';
  import Todo from './todo/Todo.svelte';

  let activeTab = 'processes';
  let statusEl;
  let countEl;

  $: if (countEl) {
    countEl.textContent = `processes: ${$processes.length}`;
  }
  $: if (statusEl) {
    statusEl.textContent = $connected ? 'status: connected' : 'status: disconnected';
  }

  onMount(() => {
    connect();
  });
</script>

<h1>RemoteIO</h1>

<div class="meta">
  <span bind:this={countEl}>processes: --</span>
  <span bind:this={statusEl}>status: connecting</span>
  <Server />
</div>

<div class="tabs">
  <button class="tab" class:active={activeTab === 'processes'}
    onclick={() => activeTab = 'processes'}>Processes</button>
  <button class="tab" class:active={activeTab === 'todo'}
    onclick={() => activeTab = 'todo'}>Todo</button>
</div>

<div id="panel-processes" class="panel" class:active={activeTab === 'processes'}>
  <Process />
</div>

<div id="panel-todo" class="panel" class:active={activeTab === 'todo'}>
  <Todo />
</div>

<style>
  h1 { font-size: 1.5rem; margin-bottom: 16px; color: #58a6ff; }

  .meta {
    display: flex; gap: 16px; margin-bottom: 12px; font-size: 0.85rem;
    color: #8b949e; align-items: center;
  }
  .meta span { background: #161b22; padding: 4px 10px; border-radius: 6px; }

  .tabs { display: flex; gap: 0; margin-bottom: 16px; border-bottom: 1px solid #30363d; }
  .tab {
    padding: 8px 20px; cursor: pointer; font-size: 0.9rem; font-weight: 600;
    color: #8b949e; border-bottom: 2px solid transparent; background: none; border-top: none;
    border-left: none; border-right: none; transition: 0.15s;
  }
  .tab:hover { color: #c9d1d9; }
  .tab.active { color: #58a6ff; border-bottom-color: #58a6ff; }
  .panel { display: none; }
  .panel.active { display: block; }

  :global(*) { margin: 0; padding: 0; box-sizing: border-box; }
  :global(body) {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: #0d1117; color: #c9d1d9; padding: 20px;
  }
</style>
