<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { toast } from './toast';
  import NeuDialog from './ui/NeuDialog.svelte';

  interface Props {
    isOpen: boolean;
    onClose: () => void;
  }

  let { isOpen, onClose }: Props = $props();

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
  let isLoading = $state(true);
  let error: string | null = $state(null);

  // Form state
  let editName = $state('');
  let editCss = $state('');
  let editFields: FieldInfo[] = $state([]);
  let editTemplates: TemplateInfo[] = $state([]);

  onMount(async () => {
    if (isOpen) {
      await loadNotetypes();
    }
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
    editTemplates = editTemplates.map((t, i) => ({ ...t, ord: i }));
  }
</script>

<NeuDialog {isOpen} {onClose} title="Note Types" size="lg">
  <div class="notetype-manager">
    <div class="sidebar">
      <h3 class="sidebar-title">Note Types</h3>
      {#if isLoading}
        <p class="loading-text">Loading...</p>
      {:else if error && notetypes.length === 0}
        <p class="error-text">{error}</p>
      {:else}
        <ul class="notetype-list" role="listbox" aria-label="Note types">
          {#each notetypes as nt}
            <li
              class="notetype-item neu-subtle {selectedNotetypeId === nt.id ? 'selected' : ''}"
              onclick={() => selectNotetype(nt.id)}
              role="option"
              aria-selected={selectedNotetypeId === nt.id}
              tabindex="0"
              onkeydown={(e) => e.key === 'Enter' && selectNotetype(nt.id)}
            >
              <span class="notetype-name">{nt.name}</span>
              <span class="notetype-kind">{nt.kind}</span>
              <span class="notetype-count">{nt.card_count} cards</span>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <div class="content">
      {#if detail && !isEditing}
        <div class="detail-header">
          <h3 class="detail-title">{detail.name}</h3>
          <div class="detail-actions">
            <button class="action-btn neu-subtle neu-btn" onclick={startEdit}>Edit</button>
            <button class="action-btn neu-subtle neu-btn" onclick={renameNotetype}>Rename</button>
            <button class="action-btn danger-btn neu-subtle neu-btn" onclick={deleteNotetype}>Delete</button>
          </div>
        </div>
        
        <div class="detail-info neu-pressed">
          <p class="info-row"><strong>Type:</strong> {detail.kind}</p>
          <p class="info-row"><strong>Fields:</strong> {detail.fields.map(f => f.name).join(', ')}</p>
          <p class="info-row"><strong>Templates:</strong> {detail.templates.length}</p>
        </div>

        <div class="detail-section">
          <h4 class="section-title">CSS</h4>
          <pre class="code-block neu-pressed">{detail.css}</pre>
        </div>

        {#each detail.templates as tmpl}
          <div class="detail-section">
            <h4 class="section-title">Card {tmpl.ord + 1}: {tmpl.name}</h4>
            <div class="template-grid">
              <div class="template-side neu-pressed">
                <strong class="template-label">Front:</strong>
                <pre class="template-code">{tmpl.front_html}</pre>
              </div>
              <div class="template-side neu-pressed">
                <strong class="template-label">Back:</strong>
                <pre class="template-code">{tmpl.back_html}</pre>
              </div>
            </div>
          </div>
        {/each}
      {:else if isEditing}
        <div class="edit-form">
          <h3 class="edit-title">Edit {detail?.name}</h3>
          
          <div class="form-group">
            <label for="name" class="form-label">Name</label>
            <input type="text" id="name" bind:value={editName} class="form-input neu-pressed" />
          </div>

           <div class="form-group">
             <label for="fields-container" class="form-label">Fields</label>
             <div id="fields-container" class="fields-list">
               {#each editFields as field, i}
                 <div class="field-row">
                   <input type="text" bind:value={field.name} placeholder="Field name" class="form-input neu-pressed" />
                   <button class="remove-btn neu-subtle neu-btn" onclick={() => removeField(i)} aria-label="Remove field">✕</button>
                 </div>
               {/each}
               <button class="add-btn neu-subtle neu-btn" onclick={addField}>Add Field</button>
             </div>
           </div>

          <div class="form-group">
            <label class="form-label">Templates</label>
            <div class="templates-list">
              {#each editTemplates as tmpl, i}
                <div class="template-edit-card neu-pressed">
                  <input type="text" bind:value={tmpl.name} placeholder="Template name" class="form-input neu-pressed" />
                  <div class="template-edit-grid">
                    <textarea bind:value={tmpl.front_html} placeholder="Front template" class="form-textarea neu-pressed"></textarea>
                    <textarea bind:value={tmpl.back_html} placeholder="Back template" class="form-textarea neu-pressed"></textarea>
                  </div>
                  <button class="remove-btn neu-subtle neu-btn" onclick={() => removeTemplate(i)} aria-label="Remove template">✕</button>
                </div>
              {/each}
              <button class="add-btn neu-subtle neu-btn" onclick={addTemplate}>Add Template</button>
            </div>
          </div>

          <div class="form-group">
            <label for="css" class="form-label">CSS</label>
            <textarea id="css" bind:value={editCss} rows="10" class="form-textarea large neu-pressed"></textarea>
          </div>

          <div class="edit-actions">
            <button class="save-btn" onclick={saveEdit}>Save</button>
            <button class="cancel-btn neu-subtle neu-btn" onclick={cancelEdit}>Cancel</button>
          </div>
        </div>
      {:else}
        <p class="empty-text">Select a notetype to view details</p>
      {/if}
    </div>
  </div>
</NeuDialog>

<style>
  .notetype-manager {
    display: flex;
    gap: 20px;
    height: 500px;
  }

  .sidebar {
    width: 240px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .sidebar-title {
    font-family: var(--sans);
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-muted);
    margin: 0;
  }

  .loading-text,
  .error-text,
  .empty-text {
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-secondary);
    text-align: center;
    padding: 32px 16px;
  }

  .error-text {
    color: var(--danger);
  }

  .notetype-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow-y: auto;
    flex: 1;
  }

  .notetype-item {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px;
    cursor: pointer;
    border-radius: var(--radius-sm);
    transition: all 0.15s ease;
  }

  .notetype-item.selected {
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 1px var(--accent);
  }

  .notetype-name {
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .notetype-kind {
    font-family: var(--sans);
    font-size: 11px;
    color: var(--text-secondary);
    text-transform: capitalize;
  }

  .notetype-count {
    font-family: var(--sans);
    font-size: 11px;
    color: var(--text-muted);
  }

  .content {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .detail-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }

  .detail-title {
    font-family: var(--serif);
    font-size: 20px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .detail-actions {
    display: flex;
    gap: 8px;
  }

  .action-btn {
    padding: 8px 12px;
    font-family: var(--sans);
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .action-btn:hover {
    color: var(--text-primary);
    background: var(--bg-subtle);
  }

  .danger-btn {
    color: var(--danger);
  }

  .danger-btn:hover {
    color: var(--danger);
    background: color-mix(in srgb, var(--danger) 10%, transparent);
  }

  .detail-info {
    padding: 16px;
    border-radius: var(--radius-sm);
  }

  .info-row {
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-secondary);
    margin: 0 0 8px 0;
    line-height: 1.5;
  }

  .info-row:last-child {
    margin-bottom: 0;
  }

  .info-row strong {
    color: var(--text-primary);
  }

  .detail-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .section-title {
    font-family: var(--sans);
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-muted);
    margin: 0;
  }

  .code-block {
    padding: 12px;
    border-radius: var(--radius-sm);
    font-family: 'SF Mono', Monaco, Consolas, monospace;
    font-size: 12px;
    color: var(--text-secondary);
    overflow-x: auto;
    max-height: 192px;
    overflow-y: auto;
    margin: 0;
    white-space: pre-wrap;
    word-break: break-all;
  }

  .template-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .template-side {
    padding: 12px;
    border-radius: var(--radius-sm);
  }

  .template-label {
    display: block;
    font-family: var(--sans);
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: 8px;
  }

  .template-code {
    font-family: 'SF Mono', Monaco, Consolas, monospace;
    font-size: 11px;
    color: var(--text-secondary);
    margin: 0;
    white-space: pre-wrap;
    word-break: break-all;
  }

  .edit-form {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .edit-title {
    font-family: var(--serif);
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .form-label {
    font-family: var(--sans);
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-muted);
  }

  .form-input {
    width: 100%;
    padding: 10px 12px;
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-primary);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    outline: none;
  }

  .form-textarea {
    width: 100%;
    padding: 10px 12px;
    font-family: 'SF Mono', Monaco, Consolas, monospace;
    font-size: 12px;
    color: var(--text-primary);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    outline: none;
    resize: vertical;
    min-height: 80px;
  }

  .form-textarea.large {
    min-height: 200px;
  }

  .fields-list,
  .templates-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .field-row {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .field-row .form-input {
    flex: 1;
  }

  .template-edit-card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    border-radius: var(--radius-sm);
  }

  .template-edit-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .remove-btn {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--sans);
    font-size: 14px;
    color: var(--text-muted);
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    flex-shrink: 0;
  }

  .remove-btn:hover {
    color: var(--danger);
    background: color-mix(in srgb, var(--danger) 10%, transparent);
  }

  .add-btn {
    padding: 8px 12px;
    font-family: var(--sans);
    font-size: 12px;
    font-weight: 500;
    color: var(--accent);
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    align-self: flex-start;
  }

  .add-btn:hover {
    background: var(--accent-soft);
  }

  .edit-actions {
    display: flex;
    gap: 12px;
    margin-top: 8px;
  }

  .save-btn {
    padding: 10px 20px;
    font-family: var(--sans);
    font-size: 13px;
    font-weight: 500;
    color: white;
    background: var(--accent);
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .save-btn:hover {
    background: color-mix(in srgb, var(--accent) 90%, black);
  }

  .cancel-btn {
    padding: 10px 20px;
    font-family: var(--sans);
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }

  .cancel-btn:hover {
    background: var(--bg-subtle);
    color: var(--text-primary);
  }
</style>
