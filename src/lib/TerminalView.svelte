<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { TerminalSession } from "./terminal";

  interface Props {
    session: TerminalSession;
    isActive: boolean;
  }

  let { session, isActive }: Props = $props();
  let container: HTMLDivElement | null = $state(null);

  onMount(async () => {
    if (container) {
      await session.mount(container);
    }
  });

  $effect(() => {
    if (isActive) {
      // Small timeout to ensure DOM layout / dimensions are updated
      requestAnimationFrame(() => {
        session.fit();
        session.terminal.focus();
      });
    }
  });
</script>

<div
  class="terminal-wrapper"
  class:active={isActive}
  style="display: {isActive ? 'block' : 'none'};"
>
  <div class="terminal-container" bind:this={container}></div>
</div>

<style>
  .terminal-wrapper {
    width: 100%;
    height: 100%;
    overflow: hidden;
    background-color: #11111b;
    scrollbar-width: none;
  }

  .terminal-container {
    width: 100%;
    height: 100%;
    scrollbar-width: none;
  }

  :global(.terminal-container .xterm) {
    padding: 6px;
    height: 100%;
  }

  :global(.terminal-container .xterm-viewport) {
    background-color: #11111b !important;
    scrollbar-width: none;
    -ms-overflow-style: none;
  }

  :global(.terminal-container .xterm-viewport::-webkit-scrollbar),
  :global(.terminal-container ::-webkit-scrollbar),
  :global(.terminal-wrapper ::-webkit-scrollbar) {
    display: none;
    width: 0;
    height: 0;
  }
</style>
