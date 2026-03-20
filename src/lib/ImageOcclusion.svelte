<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { fly, fade } from "svelte/transition";
  import { addToast } from "./toast";

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

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

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div 
  class="fixed inset-0 bg-black/50 z-50 flex items-center justify-center"
  transition:fade={{ duration: 150 }}
>
  <div 
    class="bg-white dark:bg-[#292524] rounded-2xl shadow-xl w-full max-w-4xl max-h-[90vh] overflow-hidden flex flex-col"
  >
    <!-- Header -->
    <div class="px-6 py-4 border-b border-border flex items-center justify-between">
      <h2 class="text-lg font-semibold text-text-primary">Image Occlusion</h2>
      <button
        onclick={onClose}
        class="p-2 hover:bg-bg-subtle rounded-lg transition-colors"
      >
        <svg class="h-5 w-5 text-text-secondary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>
    
    <!-- Content -->
    <div class="flex-1 overflow-auto p-6">
      {#if currentStep === "upload"}
        <div class="text-center py-8">
          <svg class="w-16 h-16 mx-auto text-text-secondary mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
          </svg>
          <p class="text-text-secondary mb-4">Upload an image to create occlusions</p>
          
          <label class="inline-flex items-center px-4 py-2 bg-accent text-white rounded-lg cursor-pointer hover:bg-accent/90">
            <input 
              type="file" 
              accept="image/*" 
              onchange={handleFileUpload}
              class="hidden"
            />
            Choose Image
          </label>
        </div>
        
      {:else if currentStep === "draw"}
        <div class="space-y-4">
          <p class="text-text-secondary">
            Draw rectangles over the areas you want to occlude. Each rectangle becomes a separate card.
          </p>
          
          <div class="flex gap-2 mb-2">
            <button
              onclick={undoLastRect}
              class="px-3 py-1.5 bg-bg-subtle text-text-primary rounded-lg text-sm hover:bg-bg-active"
            >
              Undo
            </button>
            <button
              onclick={clearAll}
              class="px-3 py-1.5 bg-bg-subtle text-text-primary rounded-lg text-sm hover:bg-bg-active"
            >
              Clear All
            </button>
            <span class="flex-1"></span>
            <span class="text-sm text-text-secondary">{rectangles.length} occlusion(s)</span>
          </div>
          
          <div class="overflow-auto max-h-[50vh] border border-border rounded-lg">
            <canvas
              bind:this={canvasEl}
              onmousedown={handleMouseDown}
              onmousemove={handleMouseMove}
              onmouseup={handleMouseUp}
              onmouseleave={handleMouseUp}
              class="max-w-full cursor-crosshair"
              style="display: block;"
            ></canvas>
          </div>
          
          <div class="flex justify-end gap-2">
            <button
              onclick={() => currentStep = "upload"}
              class="px-4 py-2 border border-border rounded-lg text-text-primary hover:bg-bg-subtle"
            >
              Back
            </button>
            <button
              onclick={proceedToDetails}
              class="px-4 py-2 bg-accent text-white rounded-lg hover:bg-accent/90"
            >
              Continue
            </button>
          </div>
        </div>
        
      {:else if currentStep === "details"}
        <div class="space-y-4 max-w-md mx-auto">
          <div>
            <label class="block text-sm font-medium text-text-secondary mb-1">Back Extra (optional)</label>
            <textarea
              bind:value={backExtra}
              rows="3"
              placeholder="Extra information to show on the back"
              class="w-full px-3 py-2 bg-bg-subtle border border-border rounded-lg text-text-primary resize-none"
            ></textarea>
          </div>
          
          <p class="text-sm text-text-secondary">
            This will create <strong>{rectangles.length}</strong> cards (one per occlusion) in the default Image Occlusion deck.
          </p>
          
          <div class="flex justify-end gap-2">
            <button
              onclick={() => currentStep = "draw"}
              class="px-4 py-2 border border-border rounded-lg text-text-primary hover:bg-bg-subtle"
            >
              Back
            </button>
            <button
              onclick={saveOcclusion}
              disabled={isSaving}
              class="px-4 py-2 bg-accent text-white rounded-lg hover:bg-accent/90 disabled:opacity-50"
            >
              {isSaving ? 'Saving...' : 'Create Cards'}
            </button>
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>
