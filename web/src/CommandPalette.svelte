<script lang="ts" module>
  export interface Command {
    id: string;
    label: string;
    detail: string;
    icon: string;
    run: () => void;
  }
</script>

<script lang="ts">
  import Icon from './Icon.svelte';
  import { onMount } from 'svelte';
  let { commands, onclose }: { commands: Command[]; onclose: () => void } = $props();
  let query = $state('');
  let index = $state(0);
  let list: HTMLDivElement;
  let search: HTMLInputElement;
  onMount(() => search.focus());
  $effect(() => {
    void index;
    void query;
    list?.querySelector('.active')?.scrollIntoView({ block: 'nearest' });
  });
  const filtered = $derived(
    commands.filter((c) => `${c.label} ${c.detail}`.toLowerCase().includes(query.toLowerCase())),
  );
  function execute(i: number) {
    const command = filtered[i];
    if (command) {
      onclose();
      command.run();
    }
  }
</script>

<div class="command-search">
  <Icon name="search" size={20} /><input
    bind:this={search}
    type="search"
    role="combobox"
    aria-autocomplete="list"
    aria-expanded="true"
    aria-controls="command-results"
    aria-activedescendant={filtered[index] ? `command-${filtered[index].id}` : undefined}
    aria-label="Search commands"
    placeholder="What would you like to do?"
    bind:value={query}
    oninput={() => (index = 0)}
    onkeydown={(e) => {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        index = (index + 1) % Math.max(1, filtered.length);
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        index = (index - 1 + filtered.length) % Math.max(1, filtered.length);
      }
      if (e.key === 'Enter') {
        e.preventDefault();
        execute(index);
      }
    }}
  /><button class="icon-button" onclick={onclose} aria-label="Close commands"
    ><Icon name="close" size={16} /></button
  >
</div>
<div
  class="command-list"
  id="command-results"
  role="listbox"
  aria-label="Available commands"
  bind:this={list}
>
  {#each filtered as command, i}<button
      role="option"
      id={`command-${command.id}`}
      aria-selected={i === index}
      class:active={i === index}
      onclick={() => execute(i)}
      ><Icon name={command.icon} size={17} /><span
        ><strong>{command.label}</strong><small>{command.detail}</small></span
      ><Icon name="arrow" size={14} /></button
    >{/each}{#if !filtered.length}<p>
      No matching commands. Try “scene”, “export”, or “measure”.
    </p>{/if}
</div>
<div class="command-footer">
  <span><kbd>↑ ↓</kbd> navigate</span><span><kbd>ENTER</kbd> run</span><span
    ><kbd>ESC</kbd> close</span
  >
</div>
