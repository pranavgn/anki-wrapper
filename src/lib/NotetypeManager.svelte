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

<div class="notetype-manager">
  <div class="header">
    <h2>Note Types</h2>
    <button class="btn-close" onclick={() => window.dispatchEvent(new CustomEvent('close-modal'))}>✕</button>
  </div>

  <div class="content">
    <div class="notetype-list">
      <h3>Note Types</h3>
      {#if isLoading}
        <p class="loading">Loading...</p>
      {:else if error && notetypes.length === 0}
        <p class="error">{error}</p>
      {:else}
        <ul>
          {#each notetypes as nt}
            <li 
              class:selected={selectedNotetypeId === nt.id}
              onclick={() => selectNotetype(nt.id)}
            >
              <span class="name">{nt.name}</span>
              <span class="kind">{nt.kind}</span>
              <span class="count">{nt.card_count} cards</span>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <div class="notetype-detail">
      {#if detail && !isEditing}
        <div class="detail-header">
          <h3>{detail.name}</h3>
          <div class="detail-actions">
            <button class="btn" onclick={startEdit}>Edit</button>
            <button class="btn" onclick={renameNotetype}>Rename</button>
            <button class="btn btn-danger" onclick={deleteNotetype}>Delete</button>
          </div>
        </div>
        
        <div class="detail-info">
          <p><strong>Type:</strong> {detail.kind}</p>
          <p><strong>Fields:</strong> {detail.fields.map(f => f.name).join(', ')}</p>
          <p><strong>Templates:</strong> {detail.templates.length}</p>
        </div>

        <div class="detail-section">
          <h4>CSS</h4>
          <pre class="css-preview">{detail.css}</pre>
        </div>

        {#each detail.templates as tmpl}
          <div class="detail-section">
            <h4>Card {tmpl.ord + 1}: {tmpl.name}</h4>
            <div class="template-preview">
              <div class="front">
                <strong>Front:</strong>
                <pre>{tmpl.front_html}</pre>
              </div>
              <div class="back">
                <strong>Back:</strong>
                <pre>{tmpl.back_html}</pre>
              </div>
            </div>
          </div>
        {/each}
      {:else if isEditing}
        <div class="edit-form">
          <h3>Edit {detail?.name}</h3>
          
          <div class="form-group">
            <label for="name">Name</label>
            <input type="text" id="name" bind:value={editName} />
          </div>

          <div class="form-group">
            <label>Fields</label>
            <div class="fields-list">
              {#each editFields as field, i}
                <div class="field-item">
                  <input type="text" bind:value={field.name} placeholder="Field name" />
                  <button class="btn-icon" onclick={() => removeField(i)}>✕</button>
                </div>
              {/each}
              <button class="btn" onclick={addField}>Add Field</button>
            </div>
          </div>

          <div class="form-group">
            <label>Templates</label>
            <div class="templates-list">
              {#each editTemplates as tmpl, i}
                <div class="template-item">
                  <input type="text" bind:value={tmpl.name} placeholder="Template name" />
                  <div class="template-editor">
                    <textarea bind:value={tmpl.front_html} placeholder="Front template"></textarea>
                    <textarea bind:value={tmpl.back_html} placeholder="Back template"></textarea>
                  </div>
                  <button class="btn-icon" onclick={() => removeTemplate(i)}>✕</button>
                </div>
              {/each}
              <button class="btn" onclick={addTemplate}>Add Template</button>
            </div>
          </div>

          <div class="form-group">
            <label for="css">CSS</label>
            <textarea id="css" bind:value={editCss} rows="10"></textarea>
          </div>

          <div class="form-actions">
            <button class="btn btn-primary" onclick={saveEdit}>Save</button>
            <button class="btn" onclick={cancelEdit}>Cancel</button>
          </div>
        </div>
      {:else}
        <p class="placeholder">Select a notetype to view details</p>
      {/if}
    </div>
  </div>
</div>

<Toast />

<style>
  .notetype-manager {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-primary);
    color: var(--text-primary);
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    border-bottom: 1px solid var(--border-color);
  }

  .header h2 {
    margin: 0;
  }

  .btn-close {
    background: none;
    border: none;
    font-size: 1.5rem;
    cursor: pointer;
    color: var(--text-secondary);
  }

  .content {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .notetype-list {
    width: 280px;
    border-right: 1px solid var(--border-color);
    overflow-y: auto;
    padding: 1rem;
  }

  .notetype-list h3 {
    margin-top: 0;
  }

  .notetype-list ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .notetype-list li {
    padding: 0.75rem;
    cursor: pointer;
    border-radius: 4px;
    margin-bottom: 0.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .notetype-list li:hover {
    background: var(--bg-hover);
  }

  .notetype-list li.selected {
    background: var(--bg-active);
  }

  .notetype-list .name {
    font-weight: 500;
  }

  .notetype-list .kind {
    font-size: 0.75rem;
    color: var(--text-secondary);
    text-transform: capitalize;
  }

  .notetype-list .count {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .notetype-detail {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
  }

  .detail-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }

  .detail-header h3 {
    margin: 0;
  }

  .detail-actions {
    display: flex;
    gap: 0.5rem;
  }

  .detail-info {
    margin-bottom: 1rem;
    padding: 1rem;
    background: var(--bg-secondary);
    border-radius: 4px;
  }

  .detail-info p {
    margin: 0.25rem 0;
  }

  .detail-section {
    margin-bottom: 1rem;
  }

  .detail-section h4 {
    margin: 0 0 0.5rem 0;
  }

  .css-preview {
    background: var(--bg-secondary);
    padding: 1rem;
    border-radius: 4px;
    overflow-x: auto;
    font-size: 0.875rem;
    max-height: 200px;
    overflow-y: auto;
  }

  .template-preview {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  .template-preview .front,
  .template-preview .back {
    background: var(--bg-secondary);
    padding: 1rem;
    border-radius: 4px;
  }

  .template-preview strong {
    display: block;
    margin-bottom: 0.5rem;
  }

  .template-preview pre {
    margin: 0;
    font-size: 0.75rem;
    white-space: pre-wrap;
    word-break: break-all;
  }

  .edit-form {
    max-width: 800px;
  }

  .edit-form h3 {
    margin-top: 0;
  }

  .form-group {
    margin-bottom: 1rem;
  }

  .form-group label {
    display: block;
    margin-bottom: 0.5rem;
    font-weight: 500;
  }

  .form-group input,
  .form-group textarea {
    width: 100%;
    padding: 0.5rem;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    background: var(--bg-secondary);
    color: var(--text-primary);
    font-family: inherit;
    font-size: 0.875rem;
  }

  .form-group textarea {
    resize: vertical;
  }

  .fields-list,
  .templates-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .field-item,
  .template-item {
    display: flex;
    gap: 0.5rem;
    align-items: flex-start;
  }

  .field-item input {
    flex: 1;
  }

  .template-item {
    flex-direction: column;
    background: var(--bg-secondary);
    padding: 0.5rem;
    border-radius: 4px;
  }

  .template-item input {
    margin-bottom: 0.5rem;
  }

  .template-editor {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
    width: 100%;
  }

  .template-editor textarea {
    min-height: 80px;
    font-family: monospace;
    font-size: 0.75rem;
  }

  .btn-icon {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 0.25rem;
    font-size: 1rem;
  }

  .btn-icon:hover {
    color: var(--text-danger);
  }

  .form-actions {
    display: flex;
    gap: 0.5rem;
    margin-top: 1rem;
  }

  .btn {
    padding: 0.5rem 1rem;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    background: var(--bg-secondary);
    color: var(--text-primary);
    cursor: pointer;
    font-size: 0.875rem;
  }

  .btn:hover {
    background: var(--bg-hover);
  }

  .btn-primary {
    background: var(--color-primary);
    border-color: var(--color-primary);
    color: white;
  }

  .btn-primary:hover {
    background: var(--color-primary-dark);
  }

  .btn-danger {
    color: var(--text-danger);
  }

  .btn-danger:hover {
    background: var(--bg-danger);
  }

  .loading,
  .error,
  .placeholder {
    color: var(--text-secondary);
    text-align: center;
    padding: 2rem;
  }

  .error {
    color: var(--text-danger);
  }
</style>
