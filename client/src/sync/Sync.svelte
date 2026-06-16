<script>
  import { syncState, send } from '../lib/ws.js';

  $: ({ syncing, direction, lastResult } = $syncState);

  function trigger(dir) {
    send({ type: dir === 'push' ? 'sync_push' : 'sync_pull' });
  }
</script>

<div class="sync">
  <div class="actions">
    <button class="btn" class:disabled={syncing} on:click={() => trigger('push')}>
      {#if syncing && direction === 'push'}Syncing...{:else}Push → Remote{/if}
    </button>
    <button class="btn" class:disabled={syncing} on:click={() => trigger('pull')}>
      {#if syncing && direction === 'pull'}Syncing...{:else}Pull ← Remote{/if}
    </button>
  </div>

  <div class="status">
    {#if syncing}
      <span class="syncing">Syncing {direction === 'push' ? '→' : '←'} ...</span>
    {:else}
      <span class="idle">Idle</span>
    {/if}
  </div>

  {#if lastResult}
    <div class="result status-{lastResult.status}">
      <span class="icon">{lastResult.status === 'ok' ? '✓' : '✗'}</span>
      <span class="detail">
        <strong>{lastResult.direction === 'push' ? 'Push' : 'Pull'}</strong>
        — {lastResult.summary}
        <span class="time">{lastResult.timestamp}</span>
      </span>
    </div>
  {/if}
</div>

<style>
  .sync {
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 8px;
    padding: 24px;
  }

  .actions {
    display: flex;
    gap: 12px;
    margin-bottom: 16px;
  }

  .btn {
    padding: 10px 24px;
    border: 1px solid #30363d;
    border-radius: 6px;
    background: #21262d;
    color: #c9d1d9;
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
    transition: 0.15s;
  }
  .btn:hover:not(.disabled) { background: #30363d; border-color: #58a6ff; }
  .btn.disabled { opacity: 0.5; cursor: not-allowed; }

  .status {
    margin-bottom: 12px;
    font-size: 0.85rem;
  }
  .syncing { color: #d29922; }
  .idle { color: #8b949e; }

  .result {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 0.85rem;
  }
  .result.status-ok { background: #1f6f2b33; color: #7ee787; }
  .result.status-error { background: #da363333; color: #ff7b72; }

  .icon { font-weight: 700; font-size: 1.1rem; }
  .time { color: #8b949e; margin-left: 8px; font-size: 0.75rem; }
</style>
