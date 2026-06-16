<script>
  import { connected, shuttingDown, send } from '../lib/ws.js';

  let btn;

  function handleClick() {
    if (confirm('Are you sure you want to shut down the server?')) {
      send({ type: 'shutdown' });
      btn.disabled = true;
      btn.textContent = 'Shutting down...';
    }
  }

  $: if ($shuttingDown && btn) {
    btn.disabled = true;
    btn.textContent = 'Server offline';
  }
</script>

<button class="shutdown-btn" bind:this={btn}
  disabled={!$connected} onclick={handleClick}>
  Shutdown Server
</button>

<style>
  .shutdown-btn {
    margin-left: auto; padding: 4px 14px; font-size: 0.8rem;
    background: #da3633; color: #fff; border: none; border-radius: 6px;
    cursor: pointer; font-weight: 600;
  }
  .shutdown-btn:hover:not(:disabled) { background: #f85149; }
  .shutdown-btn:disabled { background: #484f58; cursor: default; }
</style>
