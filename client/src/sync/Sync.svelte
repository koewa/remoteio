<script>
  import { onMount } from 'svelte';
  import { syncState, syncTargets, send } from '../lib/ws.js';

  let targets = [];
  let syncing = false;
  let syncingName = null;
  let syncingDir = null;
  let lastResult = null;

  $: targets = $syncTargets;

  $: if ($syncState) {
    syncing = $syncState.syncing;
    syncingName = $syncState.name;
    syncingDir = $syncState.direction;
    lastResult = $syncState.lastResult;
  }

  onMount(() => {
    send({ type: 'sync_list' });
  });

  function trigger(dir, name) {
    send({ type: dir === 'push' ? 'sync_push' : 'sync_pull', name });
  }

  function epLabel(ep) {
    let s = '';
    if (ep.user) s += ep.user + '@';
    if (ep.host) s += ep.host + ':';
    s += ep.path;
    return s;
  }
</script>

<div class="sync">
  <div class="header">
    <h2>Sync Targets</h2>
    <span class="state-label" class:active={syncing}>
      {#if syncing}
        Syncing {syncingName} ({syncingDir === 'push' ? '→' : '←'}) ...
      {:else}
        Idle
      {/if}
    </span>
  </div>

  {#if targets.length === 0}
    <p class="empty">
      No sync targets configured.<br>
      Create <code>sync_config.json</code> in the server directory with one or more targets.
    </p>
  {/if}

  {#each targets as target (target.name)}
    <div class="card">
      <div class="card-header">
        <strong>{target.name}</strong>
        <span class="path">{epLabel(target.local)}</span>
        <span class="arrow">⇄</span>
        <span class="path">{epLabel(target.remote)}</span>
      </div>
      <div class="card-actions">
        <button class="btn" class:disabled={syncing}
          on:click={() => trigger('push', target.name)}>
          {#if syncing && syncingName === target.name && syncingDir === 'push'}...{:else}Push →{/if}
        </button>
        <button class="btn" class:disabled={syncing}
          on:click={() => trigger('pull', target.name)}>
          {#if syncing && syncingName === target.name && syncingDir === 'pull'}...{:else}Pull ←{/if}
        </button>
      </div>
    </div>
  {/each}

  {#if lastResult}
    <div class="result status-{lastResult.status}">
      <span class="icon">{lastResult.status === 'ok' ? '✓' : '✗'}</span>
      <span class="detail">
        {#if lastResult.name}<strong>{lastResult.name}</strong> — {/if}
        {lastResult.direction === 'push' ? 'Push' : 'Pull'}
        — {lastResult.summary}
        <span class="time">{lastResult.timestamp}</span>
      </span>
    </div>
  {/if}
</div>

<style>
  .sync { max-width: 640px; }

  .header {
    display: flex; align-items: center; gap: 12px; margin-bottom: 16px;
  }
  .header h2 { font-size: 1.1rem; color: #c9d1d9; }
  .state-label {
    font-size: 0.8rem; color: #8b949e; padding: 2px 8px;
    border-radius: 4px; background: #161b22;
  }
  .state-label.active { color: #d29922; }

  .empty {
    color: #8b949e; font-size: 0.9rem; padding: 24px 0; line-height: 1.6;
  }
  .empty code { background: #161b22; padding: 1px 5px; border-radius: 3px; }

  .card {
    background: #161b22; border: 1px solid #30363d; border-radius: 8px;
    padding: 16px; margin-bottom: 12px;
  }
  .card-header {
    display: flex; align-items: center; gap: 8px; margin-bottom: 12px;
    font-size: 0.85rem; flex-wrap: wrap;
  }
  .card-header strong { font-size: 0.95rem; color: #58a6ff; margin-right: 8px; }
  .path { color: #8b949e; font-family: monospace; font-size: 0.8rem; }
  .arrow { color: #484f58; font-size: 0.9rem; }

  .card-actions { display: flex; gap: 8px; }

  .btn {
    padding: 6px 16px; border: 1px solid #30363d; border-radius: 6px;
    background: #21262d; color: #c9d1d9; font-size: 0.85rem;
    font-weight: 600; cursor: pointer; transition: 0.15s;
  }
  .btn:hover:not(.disabled) { background: #30363d; border-color: #58a6ff; }
  .btn.disabled { opacity: 0.4; cursor: not-allowed; }

  .result {
    display: flex; align-items: center; gap: 8px;
    padding: 8px 12px; border-radius: 6px; font-size: 0.85rem; margin-top: 12px;
  }
  .result.status-ok { background: #1f6f2b33; color: #7ee787; }
  .result.status-error { background: #da363333; color: #ff7b72; }

  .icon { font-weight: 700; font-size: 1.1rem; }
  .time { color: #8b949e; margin-left: 8px; font-size: 0.75rem; }
</style>
