<script lang="ts">
  import type { CheckProgress } from "./types";

  export let progress: CheckProgress;
</script>

<div class="check-strip">
  <div class="strip-line">
    <span class="strip-what">Checking <span class="strip-name">{progress.current}</span></span>
    <span class="strip-count">{progress.done} of {progress.total}</span>
  </div>
  <div
    class="strip-bar"
    role="progressbar"
    aria-label="Check progress"
    aria-valuemin="0"
    aria-valuemax={progress.total}
    aria-valuenow={progress.done}
    aria-valuetext="{progress.done} of {progress.total}"
  >
    <div class="strip-fill" style:width="{(progress.done / progress.total) * 100}%"></div>
  </div>
</div>

<style>
  .check-strip {
    flex-shrink: 0;
    padding: 8px 16px 9px;
    background: #ffffff;
    border-bottom: 1px solid #e3e3e0;
    font-size: 13px;
    line-height: 15px;
  }
  .strip-line {
    display: flex;
    align-items: baseline;
    gap: 12px;
  }
  .strip-what {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: #3d3d45;
  }
  .strip-name {
    font-weight: 500;
    color: #1b1b1f;
  }
  .strip-count {
    flex-shrink: 0;
    color: #3d3d45;
    font-variant-numeric: tabular-nums;
  }
  .strip-bar {
    margin-top: 6px;
    height: 4px;
    border-radius: 2px;
    background: #eeeeec;
    overflow: hidden;
  }
  .strip-fill {
    height: 100%;
    background: #5b5b66;
    border-radius: 2px;
    transition: width 0.15s ease;
  }
  @media (prefers-reduced-motion: reduce) {
    .strip-fill {
      transition: none;
    }
  }
</style>
