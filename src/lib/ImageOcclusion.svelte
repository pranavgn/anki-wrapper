<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { addToast } from "./toast";
  import NeuDialog from "./ui/NeuDialog.svelte";

  interface Props {
    isOpen: boolean;
    onClose: () => void;
  }

  let { isOpen, onClose }: Props = $props();

  // State
  let currentStep = $state<"upload" | "draw" | "details">("upload");
  let imageUrl = $state("");
  let imageData = $state(""); // base64
  let imageWidth = $state(0);
  let imageHeight = $state(0);
  let canvasEl = $state<HTMLCanvasElement | null>(null);
  let ctx = $state<CanvasRenderingContext2D | null>(null);
  
  // Drawing state
  let isDrawing = $state(false);
  let startX = $state(0);
  let startY = $state(0);
  let rectangles = $state<Array<{x: number; y: number; width: number; height: number}>>([]);
  let currentRect = $state<{x: number; y: number; width: number; height: number} | null>(null);
  
  // Details form
  let backExtra = $state("");
  let isSaving = $state(false);
  
  // Handle file upload
  function handleFileUpload(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    
    const reader = new FileReader();
    reader.onload = (event) => {
      imageData = event.target?.result as string;
      imageUrl = imageData;
      currentStep = "draw";
      
      // Load image to get dimensions
      const img = new Image();
      img.onload = () => {
        imageWidth = img.width;
        imageHeight = img.height;
        
        // Set canvas size
        if (canvasEl) {
          canvasEl.width = img.width;
          canvasEl.height = img.height;
          ctx = canvasEl.getContext("2d");
          
          // Draw the image
          ctx?.drawImage(img, 0, 0);
        }
      };
      img.src = imageData;
    };
    reader.readAsDataURL(file);
  }
  
  // Canvas mouse handlers
  function handleMouseDown(e: MouseEvent) {
    if (!canvasEl) return;
    const rect = canvasEl.getBoundingClientRect();
    const scaleX = canvasEl.width / rect.width;
    const scaleY = canvasEl.height / rect.height;
    
    startX = (e.clientX - rect.left) * scaleX;
    startY = (e.clientY - rect.top) * scaleY;
    isDrawing = true;
  }
  
  function handleMouseMove(e: MouseEvent) {
    if (!isDrawing || !canvasEl || !ctx) return;
    const rect = canvasEl.getBoundingClientRect();
    const scaleX = canvasEl.width / rect.width;
    const scaleY = canvasEl.height / rect.height;
    
    const currentX = (e.clientX - rect.left) * scaleX;
    const currentY = (e.clientY - rect.top) * scaleY;
    
    const x = Math.min(startX, currentX);
    const y = Math.min(startY, currentY);
    const width = Math.abs(currentX - startX);
    const height = Math.abs(currentY - startY);
    
    currentRect = { x, y, width, height };
    
    // Redraw image and rectangles
    redrawCanvas();
    
    // Draw current rectangle
    if (currentRect && ctx) {
      ctx.strokeStyle = "#ff6b6b";
      ctx.lineWidth = 3;
      ctx.strokeRect(currentRect.x, currentRect.y, currentRect.width, currentRect.height);
      ctx.fillStyle = "rgba(255, 107, 107, 0.3)";
      ctx.fillRect(currentRect.x, currentRect.y, currentRect.width, currentRect.height);
    }
  }
  
  function handleMouseUp() {
    if (currentRect && currentRect.width > 5 && currentRect.height > 5) {
      rectangles = [...rectangles, currentRect];
    }
    isDrawing = false;
    currentRect = null;
  }
  
  function redrawCanvas() {
    if (!ctx || !canvasEl) return;
    
    // Reload image
    const img = new Image();
    img.onload = () => {
      ctx!.drawImage(img, 0, 0);
      
      // Draw all rectangles
      ctx!.strokeStyle = "#ff6b6b";
      ctx!.lineWidth = 3;
      ctx!.fillStyle = "rgba(255, 107, 107, 0.3)";
      
      rectangles.forEach((r, i) => {
        ctx!.strokeRect(r.x, r.y, r.width, r.height);
        ctx!.fillRect(r.x, r.y, r.width, r.height);
        
        // Draw number
        ctx!.fillStyle = "#fff";
        ctx!.font = "bold 20px sans-serif";
        ctx!.fillText(`${i + 1}`, r.x + 5, r.y + 20);
        ctx!.fillStyle = "rgba(255, 107, 107, 0.3)";
      });
    };
    img.src = imageUrl;
  }
  
  function undoLastRect() {
    if (rectangles.length > 0) {
      rectangles = rectangles.slice(0, -1);
      redrawCanvas();
    }
  }
  
  function clearAll() {
    rectangles = [];
    redrawCanvas();
  }
  
  function proceedToDetails() {
    if (rectangles.length === 0) {
      addToast("Please draw at least one occlusion region", "warning");
      return;
    }
    currentStep = "details";
  }
  
  async function saveOcclusion() {
    if (rectangles.length === 0) {
      addToast("Please draw at least one occlusion region", "warning");
      return;
    }
    
    isSaving = true;
    
    try {
      // Convert rectangles to the format expected by Anki's image occlusion
      // Format: rect:left=X:top=Y:width=W:height=H
      const occlusionLines = rectangles.map(r => {
        const left = (r.x / imageWidth) * 100;
        const top = (r.y / imageHeight) * 100;
        const width = (r.width / imageWidth) * 100;
        const height = (r.height / imageHeight) * 100;
        return `rect:left=${left.toFixed(1)}:top=${top.toFixed(1)}:width=${width.toFixed(1)}:height=${height.toFixed(1)}`;
      }).join('\n');
      
      // Extract filename from the image data URL
      let filename = "occlusion.png";
      if (imageUrl.startsWith('data:')) {
        const match = imageUrl.match(/data:image\/([^;]+);base64,/);
        if (match) {
          const ext = match[1] === 'jpeg' ? 'jpg' : match[1];
          filename = `io_${Date.now()}.${ext}`;
        }
      }
      
      // Extract base64 data (remove data URL prefix)
      const base64Data = imageData.split(',')[1] || imageData;
      
      const noteId = await invoke<number>("add_image_occlusion", {
        imageData: base64Data,
        filename: filename,
        occlusionData: occlusionLines,
        header: "",
        backExtra: backExtra,
        tags: ["image-occlusion"],
      });
      
      addToast(`Created occlusion note with ${rectangles.length} cards`, "success");
      onClose();
    } catch (e) {
      addToast(`Failed to save: ${e}`, "error");
    } finally {
      isSaving = false;
    }
  }
</script>

<NeuDialog {isOpen} {onClose} title="Image Occlusion" size="lg">
  <div class="occlusion-content">
    {#if currentStep === "upload"}
      <div class="upload-step">
        <svg class="upload-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
        </svg>
        <p class="upload-text">Upload an image to create occlusions</p>
        
        <label class="upload-btn">
          <input 
            type="file" 
            accept="image/*" 
            onchange={handleFileUpload}
            class="hidden-input"
          />
          Choose Image
        </label>
      </div>
      
    {:else if currentStep === "draw"}
      <div class="draw-step">
        <p class="draw-instruction">
          Draw rectangles over the areas you want to occlude. Each rectangle becomes a separate card.
        </p>
        
        <div class="draw-toolbar">
          <button
            onclick={undoLastRect}
            class="tool-btn neu-subtle"
          >
            Undo
          </button>
          <button
            onclick={clearAll}
            class="tool-btn neu-subtle"
          >
            Clear All
          </button>
          <span class="occlusion-count">{rectangles.length} occlusion(s)</span>
        </div>
        
        <div class="canvas-container neu-pressed">
          <canvas
            bind:this={canvasEl}
            onmousedown={handleMouseDown}
            onmousemove={handleMouseMove}
            onmouseup={handleMouseUp}
            onmouseleave={handleMouseUp}
            class="canvas"
          ></canvas>
        </div>
        
        <div class="draw-actions">
          <button
            onclick={() => currentStep = "upload"}
            class="action-btn neu-subtle"
          >
            Back
          </button>
          <button
            onclick={proceedToDetails}
            class="action-btn primary-btn"
          >
            Continue
          </button>
        </div>
      </div>
      
    {:else if currentStep === "details"}
      <div class="details-step">
        <div class="form-group">
          <label class="form-label">Back Extra (optional)</label>
          <textarea
            bind:value={backExtra}
            rows="3"
            placeholder="Extra information to show on the back"
            class="form-textarea neu-pressed"
          ></textarea>
        </div>
        
        <p class="card-count-info">
          This will create <strong>{rectangles.length}</strong> cards (one per occlusion) in the default Image Occlusion deck.
        </p>
        
        <div class="details-actions">
          <button
            onclick={() => currentStep = "draw"}
            class="action-btn neu-subtle"
          >
            Back
          </button>
          <button
            onclick={saveOcclusion}
            disabled={isSaving}
            class="action-btn primary-btn"
          >
            {isSaving ? 'Saving...' : 'Create Cards'}
          </button>
        </div>
      </div>
    {/if}
  </div>
</NeuDialog>

<style>
  .occlusion-content {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .upload-step {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 48px 24px;
    text-align: center;
  }

  .upload-icon {
    width: 64px;
    height: 64px;
    color: var(--text-muted);
    margin-bottom: 16px;
  }

  .upload-text {
    font-family: var(--sans);
    font-size: 14px;
    color: var(--text-secondary);
    margin: 0 0 20px 0;
  }

  .upload-btn {
    display: inline-flex;
    align-items: center;
    padding: 10px 20px;
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 500;
    color: white;
    background: var(--accent);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .upload-btn:hover {
    background: color-mix(in srgb, var(--accent) 90%, black);
  }

  .hidden-input {
    display: none;
  }

  .draw-step {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .draw-instruction {
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.5;
  }

  .draw-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .tool-btn {
    padding: 6px 12px;
    font-family: var(--sans);
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .tool-btn:hover {
    color: var(--text-primary);
    background: var(--bg-subtle);
  }

  .occlusion-count {
    flex: 1;
    text-align: right;
    font-family: var(--sans);
    font-size: 12px;
    color: var(--text-muted);
  }

  .canvas-container {
    overflow: auto;
    max-height: 50vh;
    border-radius: var(--radius-sm);
    padding: 8px;
  }

  .canvas {
    max-width: 100%;
    cursor: crosshair;
    display: block;
  }

  .draw-actions,
  .details-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .action-btn {
    padding: 10px 20px;
    font-family: var(--sans);
    font-size: 13px;
    font-weight: 500;
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .action-btn.neu-subtle {
    color: var(--text-secondary);
  }

  .action-btn.neu-subtle:hover {
    color: var(--text-primary);
    background: var(--bg-subtle);
  }

  .primary-btn {
    color: white;
    background: var(--accent);
  }

  .primary-btn:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 90%, black);
  }

  .primary-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .details-step {
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-width: 480px;
    margin: 0 auto;
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

  .form-textarea {
    width: 100%;
    padding: 10px 12px;
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-primary);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    outline: none;
    resize: none;
  }

  .form-textarea::placeholder {
    color: var(--text-muted);
  }

  .card-count-info {
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.5;
  }

  .card-count-info strong {
    color: var(--text-primary);
  }
</style>
