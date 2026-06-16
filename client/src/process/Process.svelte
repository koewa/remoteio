<script>
  import { processes, connected, send } from '../lib/ws.js';

  let filter = '';
  let sortCol = 'pid';
  let sortDir = 1;

  function esc(s) {
    const d = document.createElement('div');
    d.textContent = s;
    return d.innerHTML;
  }

  function fmtBytes(kb) {
    if (kb >= 1048576) return (kb / 1048576).toFixed(1) + ' GB';
    if (kb >= 1024) return (kb / 1024).toFixed(1) + ' MB';
    return kb + ' KB';
  }

  function fmtCpu(sec) {
    if (sec >= 3600) return (sec / 3600).toFixed(1) + 'h';
    if (sec >= 60) return (sec / 60).toFixed(1) + 'm';
    return sec.toFixed(1) + 's';
  }

  function stateClass(s) {
    return ({ R: 'R', S: 'S', D: 'D', Z: 'Z', T: 'T' })[s] || '';
  }

  function stateLabel(s) {
    return ({ R: 'running', S: 'sleeping', D: 'disk sleep', Z: 'zombie', T: 'stopped' })[s] || s;
  }

  function toggleSort(col) {
    if (sortCol === col) sortDir *= -1;
    else { sortCol = col; sortDir = 1; }
  }

  $: filtered = $processes.filter(p =>
    p.pid.toString().includes(filter) ||
    p.name.toLowerCase().includes(filter) ||
    p.uid.toString().includes(filter)
  );

  $: sorted = [...filtered].sort((a, b) => {
    let va = a[sortCol], vb = b[sortCol];
    if (typeof va === 'string') va = va.toLowerCase();
    if (typeof vb === 'string') vb = vb.toLowerCase();
    if (va < vb) return -sortDir;
    if (va > vb) return sortDir;
    return 0;
  });

  let sortArrow = (col) => sortCol === col ? (sortDir === 1 ? ' ▴' : ' ▾') : '';
  let sortCls = (col) => sortCol === col ? 'sorted' : '';
</script>

<div class="search">
  <input type="text" id="filter" placeholder="Filter by name, pid, or user..." bind:value={filter}>
</div>

<div class="table-wrap">
  <table>
    <thead>
      <tr>
        <th class={sortCls('pid')} onclick={() => toggleSort('pid')}>PID{sortArrow('pid')}</th>
        <th class={sortCls('name')} onclick={() => toggleSort('name')}>Name{sortArrow('name')}</th>
        <th class={sortCls('state')} onclick={() => toggleSort('state')}>State{sortArrow('state')}</th>
        <th class={sortCls('uid')} onclick={() => toggleSort('uid')}>User{sortArrow('uid')}</th>
        <th class="rss {sortCls('rss')}" onclick={() => toggleSort('rss')}>RSS{sortArrow('rss')}</th>
        <th class="cpu {sortCls('cpu')}" onclick={() => toggleSort('cpu')}>CPU Time{sortArrow('cpu')}</th>
        <th class={sortCls('cmdline')} onclick={() => toggleSort('cmdline')}>Command{sortArrow('cmdline')}</th>
      </tr>
    </thead>
    <tbody>
      {#each sorted as p (p.pid)}
        <tr>
          <td class="pid">{esc(p.pid)}</td>
          <td>{esc(p.name)}</td>
          <td><span class="state {stateClass(p.state)}">{stateLabel(p.state)}</span></td>
          <td>{p.uid}</td>
          <td class="rss">{fmtBytes(p.rss_kb)}</td>
          <td class="cpu">{fmtCpu(p.cpu_time_sec)}</td>
          <td class="cmdline" title={esc(p.cmdline)}>{esc(p.cmdline)}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<div id="connecting" class="connecting" class:visible={!$connected && $processes.length === 0}>
  {#if $connected}
    Connecting to server...
  {:else}
    Reconnecting in 3 seconds...
  {/if}
</div>

<style>
  .search { margin-bottom: 12px; }
  .search input {
    width: 100%; max-width: 320px; padding: 6px 12px;
    background: #161b22; border: 1px solid #30363d; border-radius: 6px;
    color: #c9d1d9; font-size: 0.85rem; outline: none;
  }
  .search input:focus { border-color: #58a6ff; }

  .table-wrap { overflow-x: auto; }
  table {
    width: 100%; border-collapse: collapse; font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
  }
  th {
    position: sticky; top: 0; background: #161b22; text-align: left;
    padding: 8px 10px; border-bottom: 2px solid #30363d;
    color: #8b949e; font-weight: 600; cursor: pointer; user-select: none;
  }
  th:hover { color: #58a6ff; }
  :global(th.sorted) { color: #58a6ff; }
  td { padding: 4px 10px; border-bottom: 1px solid #21262d; white-space: nowrap; }
  tr:hover td { background: #161b22; }
  .pid { color: #58a6ff; font-weight: 600; }
  .state {
    display: inline-block; padding: 1px 6px; border-radius: 4px;
    font-size: 0.75rem; font-weight: 600;
  }
  :global(.R) { background: #1f6f2b; color: #7ee787; }
  :global(.S) { background: #1f6f2b44; color: #7ee787; }
  :global(.D) { background: #9e6a0344; color: #d29922; }
  :global(.Z) { background: #da363344; color: #ff7b72; }
  :global(.T) { background: #8b949e44; color: #8b949e; }
  .cmdline { max-width: 400px; overflow: hidden; text-overflow: ellipsis; }
  .rss { text-align: right; }
  .cpu { text-align: right; }

  .connecting { color: #8b949e; padding: 20px; text-align: center; display: none; }
  .connecting.visible { display: block; }

  @media (max-width: 600px) {
    .cmdline { max-width: 120px; }
    th, td { padding: 4px 6px; }
  }
</style>
