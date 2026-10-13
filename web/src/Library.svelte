<script lang="ts">
  import Icon from './Icon.svelte';
  import { onMount } from 'svelte';
  import SceneThumbnail from './SceneThumbnail.svelte';
  import type { Experiment } from './library';
  let {
    entries,
    defaultName,
    onsave,
    onload,
    onremove,
    onrename,
    onexport,
    onclose,
  }: {
    entries: Experiment[];
    defaultName: string;
    onsave: (name: string) => void;
    onload: (entry: Experiment) => void;
    onremove: (id: string) => void;
    onrename: (id: string, name: string) => void;
    onexport: (entry: Experiment) => void;
    onclose: () => void;
  } = $props();
  let name = $state('');
  onMount(() => document.getElementById('experiment-name')?.focus());
  $effect(() => {
    name = defaultName;
  });
  let search = $state('');
  let editing = $state('');
  let removing = $state('');
  let renamed = $state('');
  const filtered = $derived(
    entries.filter((e) => e.name.toLowerCase().includes(search.toLowerCase())),
  );
  const date = (n: number) =>
    new Intl.DateTimeFormat(undefined, {
      month: 'short',
      day: 'numeric',
      hour: 'numeric',
      minute: '2-digit',
    }).format(n);
</script>

<div class="dialog-header">
  <span class="eyebrow">YOUR EXPERIMENTS / {entries.length} OF 24</span><button
    class="icon-button"
    onclick={onclose}
    aria-label="Close library"><Icon name="close" /></button
  >
</div>
<h2>A place for your discoveries.</h2>
<p>Saved in this browser. Export important experiments to keep a separate copy.</p>
<form
  class="save-form"
  onsubmit={(e) => {
    e.preventDefault();
    onsave(name);
  }}
>
  <label class="visually-hidden" for="experiment-name">Experiment name</label><input
    id="experiment-name"
    bind:value={name}
    maxlength="80"
    required
    placeholder="Name this experiment"
  /><button class="primary-button" type="submit"
    ><Icon name="save" size={15} />Save current scene</button
  >
</form>
<p class="microcopy">Saving with an existing name updates that experiment.</p>
<label class="search-field"
  ><Icon name="search" size={15} /><input
    type="search"
    bind:value={search}
    aria-label="Search saved experiments"
    placeholder="Find an experiment…"
  /></label
>
<div class="library-list">
  {#each filtered as entry (entry.id)}<article class="saved-experiment">
      <SceneThumbnail scene={entry.scene} />
      <div class="saved-copy">
        {#if editing === entry.id}<form
            onsubmit={(e) => {
              e.preventDefault();
              onrename(entry.id, renamed);
              editing = '';
            }}
          >
            <input
              aria-label="New experiment name"
              bind:value={renamed}
              maxlength="80"
              required
            /><button class="text-button" type="submit">Rename</button>
          </form>{:else}<h3>{entry.name}</h3>{/if}
        <small>{entry.scene.bodies.length} bodies · {entry.scene.joints.length} joints</small><time
          datetime={new Date(entry.updatedAt).toISOString()}>{date(entry.updatedAt)}</time
        >
        <div class="saved-actions">
          <button class="text-button" onclick={() => onload(entry)}
            >Open experiment <Icon name="arrow" size={13} /></button
          ><button
            class="icon-button"
            aria-label={`Rename ${entry.name}`}
            onclick={() => {
              editing = entry.id;
              renamed = entry.name;
            }}><Icon name="edit" size={14} /></button
          ><button
            class="icon-button"
            aria-label={`Export ${entry.name}`}
            onclick={() => onexport(entry)}><Icon name="download" size={14} /></button
          ><button
            class="icon-button"
            aria-label={`Delete ${entry.name}`}
            onclick={() => {
              if (removing === entry.id) {
                onremove(entry.id);
                removing = '';
              } else removing = entry.id;
            }}
            >{#if removing === entry.id}<span class="confirm-remove">Remove?</span>{:else}<Icon
                name="trash"
                size={14}
              />{/if}</button
          >
        </div>
      </div>
    </article>{/each}
  {#if !filtered.length}<div class="library-empty">
      <Icon name="folder" size={30} />
      <h3>{entries.length ? 'No matching experiments.' : 'Your next discovery belongs here.'}</h3>
      <p>Save the current scene with a name to get started.</p>
    </div>{/if}
</div>
