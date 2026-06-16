<script>
  import { onMount } from 'svelte';
  import { syncState, syncTargets, send } from '../lib/ws.js';

  let targets = [];
  let syncing = false;
  let lastResult = null;

  let showAdd = false;
  let editing = {}; // name → true

  let form = { name: '', local: { host: '', path: '', user: '' }, remote: { host: '', path: '', user: '' } };

  $: targets = $syncTargets;

  $: if ($syncState) {
    syncing = $syncState.syncing;
    lastResult = $syncState.lastResult;
  }

  onMount(() => {
    send({ type: 'sync_list' });
  });

  function epLabel(ep) {
    let s = '';
    if (ep.user) s += ep.user + '@';
    if (ep.host) s += ep.host + ':';
    s += ep.path;
    return s;
  }

  function resetForm() {
    form = { name: '', local: { host: '', path: '', user: '' }, remote: { host: '', path: '', user: '' } };
  }

  function startAdd() {
    resetForm();
    showAdd = true;
  }

  function cancelAdd() {
    showAdd = false;
  }

  function startEdit(t) {
    form = {
      name: t.name,
      local: { ...t.local },
      remote: { ...t.remote },
    };
    editing[t.name] = true;
  }

  function cancelEdit(name) {
    editing[name] = false;
  }

  function save(name) {
    send({ type: 'sync_save', target: form });
    showAdd = false;
    if (name) editing[name] = false;
  }

  function deleteTarget(name) {
    if (!confirm(`Delete sync target "${name}"?`)) return;
    send({ type: 'sync_delete', name });
  }

  function duplicateTarget(t) {
    const copy = {
      name: t.name + ' (copy)',
      local: { ...t.local },
      remote: { ...t.remote },
    };
    send({ type: 'sync_save', target: copy });
    form = copy;
    editing[copy.name] = true;
  }
</script>

<div class="sync">
  <div class="header">
    <h2>Sync Targets</h2>
    <span class="state-label" class:active={syncing}>
      {#if syncing}Syncing ...{:else}Idle{/if}
    </span>
    <button class="btn-add" on:click={startAdd}>+ Add Target</button>
  </div>

  {#if showAdd}
    <div class="card form-card">
      <h3>New Target</h3>
      <div class="field">
        <label>Name</label>
        <input type="text" bind:value={form.name} placeholder="e.g. Main Laptop">
      </div>
      <div class="endpoint-fields">
        <div class="ep">
          <strong>Local</strong>
          <input type="text" bind:value={form.local.host} placeholder="host (empty = local)">
          <input type="text" bind:value={form.local.path} placeholder="path">
          <input type="text" bind:value={form.local.user} placeholder="user">
        </div>
        <div class="ep">
          <strong>Remote</strong>
          <input type="text" bind:value={form.remote.host} placeholder="host">
          <input type="text" bind:value={form.remote.path} placeholder="path">
          <input type="text" bind:value={form.remote.user} placeholder="user">
        </div>
      </div>
      <div class="form-actions">
        <button class="btn btn-primary" on:click={() => save()}>Save</button>
        <button class="btn" on:click={cancelAdd}>Cancel</button>
      </div>
    </div>
  {/if}

  {#if targets.length === 0 && !showAdd}
    <p class="empty">
      No sync targets configured.
    </p>
  {/if}

  {#each targets as target (target.name)}
    {#if editing[target.name]}
      <div class="card form-card">
        <h3>Edit: {target.name}</h3>
        <div class="field">
          <label>Name</label>
          <input type="text" bind:value={form.name}>
        </div>
        <div class="endpoint-fields">
          <div class="ep">
            <strong>Local</strong>
            <input type="text" bind:value={form.local.host} placeholder="host">
            <input type="text" bind:value={form.local.path} placeholder="path">
            <input type="text" bind:value={form.local.user} placeholder="user">
          </div>
          <div class="ep">
            <strong>Remote</strong>
            <input type="text" bind:value={form.remote.host} placeholder="host">
            <input type="text" bind:value={form.remote.path} placeholder="path">
            <input type="text" bind:value={form.remote.user} placeholder="user">
          </div>
        </div>
        <div class="form-actions">
          <button class="btn btn-primary" on:click={() => save(target.name)}>Save</button>
          <button class="btn" on:click={() => cancelEdit(target.name)}>Cancel</button>
        </div>
      </div>
    {:else}
      <div class="card">
        <div class="card-header">
          <strong>{target.name}</strong>
        </div>
        <div class="card-paths">
          <span class="path">{epLabel(target.local)}</span>
          <span class="arrow">⇄</span>
          <span class="path">{epLabel(target.remote)}</span>
        </div>
        <div class="card-actions">
          <button class="btn" class:disabled={syncing}
            on:click={() => send({ type: 'sync_push', name: target.name })}>
            Push →
          </button>
          <button class="btn" class:disabled={syncing}
            on:click={() => send({ type: 'sync_pull', name: target.name })}>
            Pull ←
          </button>
          <button class="btn btn-edit" on:click={() => startEdit(target)}>Edit</button>
          <button class="btn btn-copy" on:click={() => duplicateTarget(target)}>Duplicate</button>
          <button class="btn btn-del" on:click={() => deleteTarget(target.name)}>Delete</button>
        </div>
      </div>
    {/if}
  {/each}

  {#if lastResult}
    <div class="result status-{lastResult.status}">
      <span class="icon">{lastResult.status === 'ok' ? '✓' : '✗'}</span>
      <span class="detail">
        {#if lastResult.name}<strong>{lastResult.name}</strong> — {/if}
        {lastResult.direction === 'push' ? 'Push' : lastResult.direction === 'pull' ? 'Pull' : ''}
        {lastResult.summary}
        <span class="time">{lastResult.timestamp}</span>
      </span>
    </div>
  {/if}
</div>

<style>
  .sync { max-width: 700px; }

  .header {
    display: flex; align-items: center; gap: 12px; margin-bottom: 16px; flex-wrap: wrap;
  }
  .header h2 { font-size: 1.1rem; color: #c9d1d9; }
  .state-label {
    font-size: 0.8rem; color: #8b949e; padding: 2px 8px;
    border-radius: 4px; background: #161b22;
  }
  .state-label.active { color: #d29922; }

  .btn-add {
    margin-left: auto; padding: 6px 14px; border: 1px solid #238636;
    border-radius: 6px; background: #238636; color: #fff;
    font-size: 0.85rem; font-weight: 600; cursor: pointer; transition: 0.15s;
  }
  .btn-add:hover { background: #2ea043; }

  .empty { color: #8b949e; font-size: 0.9rem; padding: 24px 0; }

  .card {
    background: #161b22; border: 1px solid #30363d; border-radius: 8px;
    padding: 16px; margin-bottom: 12px;
  }
  .form-card { border-color: #58a6ff; }
  .form-card h3 { font-size: 0.95rem; color: #58a6ff; margin-bottom: 12px; }

  .card-header { margin-bottom: 6px; }
  .card-header strong { font-size: 0.95rem; color: #58a6ff; }

  .card-paths {
    display: flex; align-items: center; gap: 8px; margin-bottom: 12px;
    font-family: monospace; font-size: 0.8rem; flex-wrap: wrap;
  }
  .path { color: #8b949e; }
  .arrow { color: #484f58; }

  .card-actions { display: flex; gap: 8px; flex-wrap: wrap; }
  .form-actions { display: flex; gap: 8px; margin-top: 12px; }

  .field { margin-bottom: 12px; }
  .field label { display: block; font-size: 0.8rem; color: #8b949e; margin-bottom: 4px; }

  .endpoint-fields { display: flex; gap: 16px; flex-wrap: wrap; }
  .ep { flex: 1; min-width: 200px; }
  .ep strong { display: block; font-size: 0.8rem; color: #8b949e; margin-bottom: 4px; }

  input {
    width: 100%; padding: 6px 10px; margin-bottom: 6px;
    background: #0d1117; border: 1px solid #30363d; border-radius: 6px;
    color: #c9d1d9; font-size: 0.85rem; outline: none; box-sizing: border-box;
  }
  input:focus { border-color: #58a6ff; }

  .btn {
    padding: 6px 16px; border: 1px solid #30363d; border-radius: 6px;
    background: #21262d; color: #c9d1d9; font-size: 0.85rem;
    font-weight: 600; cursor: pointer; transition: 0.15s;
  }
  .btn:hover:not(.disabled) { background: #30363d; border-color: #58a6ff; }
  .btn.disabled { opacity: 0.4; cursor: not-allowed; }
  .btn-primary { background: #238636; border-color: #238636; color: #fff; }
  .btn-primary:hover { background: #2ea043; border-color: #2ea043; }
  .btn-edit { border-color: #d29922; color: #d29922; }
  .btn-edit:hover:not(.disabled) { background: #d2992222; }
  .btn-copy { border-color: #58a6ff; color: #58a6ff; }
  .btn-copy:hover:not(.disabled) { background: #58a6ff22; }
  .btn-del { border-color: #da3633; color: #ff7b72; }
  .btn-del:hover:not(.disabled) { background: #da363322; }

  .result {
    display: flex; align-items: center; gap: 8px;
    padding: 8px 12px; border-radius: 6px; font-size: 0.85rem; margin-top: 12px;
  }
  .result.status-ok { background: #1f6f2b33; color: #7ee787; }
  .result.status-error { background: #da363333; color: #ff7b72; }

  .icon { font-weight: 700; font-size: 1.1rem; }
  .time { color: #8b949e; margin-left: 8px; font-size: 0.75rem; }
</style>
