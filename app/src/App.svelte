<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  type Repo = {
    path: string;
    name: string;
    error: string | null;
    behind: number;
    ahead: number;
    unsaved: boolean;
  };

  function label(r: Repo): string {
    if (r.error) return r.error;
    if (r.behind > 0) return `Behind by ${r.behind}`;
    if (r.ahead > 0) return `Ahead by ${r.ahead}`;
    if (r.unsaved) return "Has unsaved changes";
    return "Up to date";
  }

  const repos = invoke<Repo[]>("repos");
</script>

<main>
  <h1>RepoBell</h1>
  {#await repos then list}
    {#if list.length === 0}
      <p class="muted">No repos yet. Add a folder in Settings.</p>
    {:else}
      <ul>
        {#each list as r (r.path)}
          <li>
            <strong>{r.name}</strong>
            <span class:err={r.error}>{label(r)}</span>
          </li>
        {/each}
      </ul>
    {/if}
  {:catch e}
    <p class="err">{e}</p>
  {/await}
</main>

<style>
  :global(body) {
    margin: 0;
    font: 14px/1.4 system-ui, sans-serif;
    color: #1f2328;
    background: #f6f8fa;
  }
  main { padding: 16px; }
  h1 { font-size: 18px; margin: 0 0 12px; }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; }
  li {
    display: flex;
    flex-direction: column;
    padding: 10px 12px;
    background: #fff;
    border: 1px solid #d0d7de;
    border-radius: 6px;
  }
  .muted { color: #656d76; }
  span { color: #656d76; }
  .err { color: #b42318; }
</style>
