<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import Toast from './Toast.svelte';
  import { toast } from './toast';

  interface NotetypeInfo {
    id: number;
    name: string;
    kind: string;
    card_count: number;
  }

  interface FieldInfo {
    ord: number;
    name: string;
  }

  interface TemplateInfo {
    ord: number;
    name: string;
    front_html: string;
    back_html: string;
  }

  interface NotetypeDetail {
    id: number;
    name: string;
    kind: string;
    css: string;
    fields: FieldInfo[];
    templates: TemplateInfo[];
  }

  let notetypes: NotetypeInfo[] = $state([]);
  let selectedNotetypeId: number | null = $state(null);
  let detail: NotetypeDetail | null = $state(null);
  let isEditing = $state(false);
  let isAddingNew = $state(false);
  let isLoading = $state(true);
  let error: string | null = $state(null);

  // Form state
  let editName = $state('');
  let editCss = $state('');
  let editFields: FieldInfo[] = $state([]);
  let editTemplates: TemplateInfo[] = $state([]);

  // New notetype
  let newNotetypeName = $state('');
  let newNotetypeKind = $state('standard');

  onMount(async () => {
    await loadNotetypes();
  });

  async function loadNotetypes() {
    isLoading = true;
    error = null;
    try {
      notetypes = await invoke<NotetypeInfo[]>('get_all_notetypes');
    } catch (e) {
      error = String(e);
      toast.error('Failed to load notetypes');
    } finally {
      isLoading = false;
    }
  }

  async function selectNotetype(id: number) {
    selectedNotetypeId = id;
    isEditing = false;
    isAddingNew = false;
    try {
      detail = await invoke<NotetypeDetail>('get_notetype_detail', { notetypeId: id });
    } catch (e) {
      error = String(e);
      toast.error('Failed to load notetype detail');
    }
  }

  function startEdit() {
    if (!detail) return;
    editName = detail.name;
    editCss = detail.css;
    editFields = detail.fields.map(f => ({ ...f }));
    editTemplates = detail.templates.map(t => ({ ...t }));
    isEditing = true;
  }

  function cancelEdit() {
    isEditing = false;
    editName = '';
    editCss = '';
    editFields = [];
    editTemplates = [];
  }

  async function saveEdit() {
    if (!detail) return;
    try {
      const updatedDetail: NotetypeDetail = {
        ...detail,
        name: editName,
        css: editCss,
        fields: editFields,
        templates: editTemplates
      };
      await invoke('update_notetype_detail', { detail: updatedDetail });
      toast.success('Notetype updated');
      isEditing = false;
      await loadNotetypes();
      if (selectedNotetypeId) {
        await selectNotetype(selectedNotetypeId);
      }
    } catch (e) {
      error = String(e);
      toast.error('Failed to update notetype');
    }
  }

  async function renameNotetype() {
    if (!detail || !selectedNotetypeId) return;
    const newName = prompt('Enter new name:', detail.name);
    if (!newName || newName === detail.name) return;
    try {
      await invoke('rename_notetype', { notetypeId: selectedNotetypeId, newName });
      toast.success('Notetype renamed');
      await loadNotetypes();
      await selectNotetype(selectedNotetypeId);
    } catch (e) {
      error = String(e);
      toast.error('Failed to rename notetype');
    }
  }

  async function deleteNotetype() {
    if (!selectedNotetypeId) return;
    if (!confirm('Are you sure you want to delete this notetype? All cards using it will be deleted!')) return;
    try {
      await invoke('delete_notetype', { notetypeId: selectedNotetypeId });
      toast.success('Notetype deleted');
      selectedNotetypeId = null;
      detail = null;
      await loadNotetypes();
    } catch (e) {
      error = String(e);
      toast.error('Failed to delete notetype');
    }
  }

  function addField() {
    const newOrd = editFields.length;
    editFields = [...editFields, { ord: newOrd, name: `Field ${newOrd + 1}` }];
  }

  function removeField(index: number) {
    editFields = editFields.filter((_, i) => i !== index);
    // Re-ord remaining fields
    editFields = editFields.map((f, i) => ({ ...f, ord: i }));
  }

  function addTemplate() {
    const newOrd = editTemplates.length;
    editTemplates = [...editTemplates, { 
      ord: newOrd, 
      name: `Card ${newOrd + 1}`, 
      front_html: '{{Front}}', 
      back_html: '{{FrontSide}}<hr id="answer">{{Back}}' 
    }];
  }

  function removeTemplate(index: number) {
    editTemplates = editTemplates.filter((_, i) => i !== index);
    // Re-ord remaining templates
    editTemplates = editTemplates.map((t, i) => ({ ...t, ord: i }));
  }
</script>

<div class="flex flex-col h-full bg-bg-base text-text-primary">
  <div class="flex justify-between items-center p-4 border-b border-border">
    <h2 class="text-lg font-semibold m-0">Note Types</h2>
    <button class="bg-transparent border-none text-2xl cursor-pointer text-text-secondary hover:text-text-primary" onclick={() => window.dispatchEvent(new CustomEvent('close-modal'))}>✕</button>
  </div>

  <div class="flex flex-1 overflow-hidden">
    <div class="w-72 border-r border-border overflow-y-auto p-4">
      <h3 class="mt-0 mb-4">Note Types</h3>
      {#if isLoading}
        <p class="text-text-secondary text-center p-8">Loading...</p>
      {:else if error && notetypes.length === 0}
        <p class="text-danger text-center p-8">{error}</p>
      {:else}
        <ul class="list-none p-0 m-0">
          {#each notetypes as nt}
            <li 
              class="p-3 cursor-pointer rounded-md mb-1 flex flex-col gap-1 hover:bg-bg-subtle {selectedNotetypeId === nt.id ? 'bg-accent-soft' : ''}"
              onclick={() => selectNotetype(nt.id)}
            >
              <span class="font-medium">{nt.name}</span>
              <span class="text-xs text-text-secondary capitalize">{nt.kind}</span>
              <span class="text-xs text-text-secondary">{nt.card_count} cards</span>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <div class="flex-1 overflow-y-auto p-4">
      {#if detail && !isEditing}
        <div class="flex justify-between items-center mb-4">
          <h3 class="m-0">{detail.name}</h3>
          <div class="flex gap-2">
            <button class="px-4 py-2 border border-border rounded-md bg-bg-subtle text-text-primary cursor-pointer text-sm hover:bg-bg-subtle" onclick={startEdit}>Edit</button>
            <button class="px-4 py-2 border border-border rounded-md bg-bg-subtle text-text-primary cursor-pointer text-sm hover:bg-bg-subtle" onclick={renameNotetype}>Rename</button>
            <button class="px-4 py-2 border border-border rounded-md bg-bg-subtle text-danger cursor-pointer text-sm hover:bg-bg-subtle" onclick={deleteNotetype}>Delete</button>
          </div>
        </div>
        
        <div class="mb-4 p-4 bg-bg-subtle rounded-md">
          <p class="m-1"><strong>Type:</strong> {detail.kind}</p>
          <p class="m-1"><strong>Fields:</strong> {detail.fields.map(f => f.name).join(', ')}</p>
          <p class="m-1"><strong>Templates:</strong> {detail.templates.length}</p>
        </div>

        <div class="mb-4">
          <h4 class="m-0 mb-2">CSS</h4>
          <pre class="bg-bg-subtle p-4 rounded-md overflow-x-auto text-sm max-h-48 overflow-y-auto">{detail.css}</pre>
        </div>

        {#each detail.templates as tmpl}
          <div class="mb-4">
            <h4 class="m-0 mb-2">Card {tmpl.ord + 1}: {tmpl.name}</h4>
            <div class="grid grid-cols-2 gap-4">
              <div class="bg-bg-subtle p-4 rounded-md">
                <strong class="block mb-2">Front:</strong>
                <pre class="m-0 text-xs whitespace-pre-wrap break-all">{tmpl.front_html}</pre>
              </div>
              <div class="bg-bg-subtle p-4 rounded-md">
                <strong class="block mb-2">Back:</strong>
                <pre class="m-0 text-xs whitespace-pre-wrap break-all">{tmpl.back_html}</pre>
              </div>
            </div>
          </div>
        {/each}
      {:else if isEditing}
        <div class="max-w-4xl">
          <h3 class="mt-0 mb-4">Edit {detail?.name}</h3>
          
          <div class="mb-4">
            <label for="name" class="block mb-2 font-medium">Name</label>
            <input type="text" id="name" bind:value={editName} class="w-full p-2 border border-border rounded-md bg-bg-subtle text-text-primary font-inherit text-sm" />
          </div>

          <div class="mb-4">
            <label class="block mb-2 font-medium">Fields</label>
            <div class="flex flex-col gap-2">
              {#each editFields as field, i}
                <div class="flex gap-2 items-start">
                  <input type="text" bind:value={field.name} placeholder="Field name" class="flex-1 p-2 border border-border rounded-md bg-bg-subtle text-text-primary font-inherit text-sm" />
                  <button class="bg-transparent border-none text-text-secondary cursor-pointer p-1 text-lg hover:text-danger" onclick={() => removeField(i)}>✕</button>
                </div>
              {/each}
              <button class="px-4 py-2 border border-border rounded-md bg-bg-subtle text-text-primary cursor-pointer text-sm hover:bg-bg-subtle" onclick={addField}>Add Field</button>
            </div>
          </div>

          <div class="mb-4">
            <label class="block mb-2 font-medium">Templates</label>
            <div class="flex flex-col gap-2">
              {#each editTemplates as tmpl, i}
                <div class="flex flex-col gap-2 bg-bg-subtle p-2 rounded-md">
                  <input type="text" bind:value={tmpl.name} placeholder="Template name" class="w-full p-2 border border-border rounded-md bg-bg-subtle text-text-primary font-inherit text-sm mb-2" />
                  <div class="grid grid-cols-2 gap-2 w-full">
                    <textarea bind:value={tmpl.front_html} placeholder="Front template" class="min-h-20 font-mono text-xs p-2 border border-border rounded-md bg-bg-subtle text-text-primary"></textarea>
                    <textarea bind:value={tmpl.back_html} placeholder="Back template" class="min-h-20 font-mono text-xs p-2 border border-border rounded-md bg-bg-subtle text-text-primary"></textarea>
                  </div>
                  <button class="bg-transparent border-none text-text-secondary cursor-pointer p-1 text-lg hover:text-danger self-end" onclick={() => removeTemplate(i)}>✕</button>
                </div>
              {/each}
              <button class="px-4 py-2 border border-border rounded-md bg-bg-subtle text-text-primary cursor-pointer text-sm hover:bg-bg-subtle" onclick={addTemplate}>Add Template</button>
            </div>
          </div>

          <div class="mb-4">
            <label for="css" class="block mb-2 font-medium">CSS</label>
            <textarea id="css" bind:value={editCss} rows="10" class="w-full p-2 border border-border rounded-md bg-bg-subtle text-text-primary font-inherit text-sm resize-y"></textarea>
          </div>

          <div class="flex gap-2 mt-4">
            <button class="px-4 py-2 border border-accent rounded-md bg-accent text-white cursor-pointer text-sm hover:bg-accent" onclick={saveEdit}>Save</button>
            <button class="px-4 py-2 border border-border rounded-md bg-bg-subtle text-text-primary cursor-pointer text-sm hover:bg-bg-subtle" onclick={cancelEdit}>Cancel</button>
          </div>
        </div>
      {:else}
        <p class="text-text-secondary text-center p-8">Select a notetype to view details</p>
      {/if}
    </div>
  </div>
</div>

<Toast />
