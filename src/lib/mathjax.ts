// MathJax initialization and rendering utilities
// Matches Anki's default MathJax configuration
// Lazy-loads MathJax only when math content is detected

let mjInitialized = false;
let mjInitializing: Promise<void> | null = null;

// Lightweight check — does the HTML contain math delimiters?
export function containsMath(html: string): boolean {
  return /\\\(|\\\[|[latex]|[\$]/.test(html);
}

export async function initMathJax(): Promise<void> {
  if (mjInitialized) return;
  if (mjInitializing) return mjInitializing;
  
  mjInitializing = (async () => {
    // MathJax configuration matching Anki's defaults
    (window as any).MathJax = {
      tex: {
        inlineMath: [['\\(', '\\)']],
        displayMath: [['\\[', '\\]']],
        processEscapes: true,
      },
      svg: { fontCache: 'global' },
      startup: { typeset: false }, // manual control
    };
    
    // Load MathJax with a timeout to prevent hanging
    try {
      const timeoutPromise = new Promise((_, reject) =>
        setTimeout(() => reject(new Error('MathJax load timeout')), 5000)
      );
      await Promise.race([import('mathjax/tex-svg.js'), timeoutPromise]);
      mjInitialized = true;
    } catch (e) {
      console.warn('MathJax initialization failed (non-critical):', e);
      // App continues to work — MathJax rendering will be skipped
    }
  })();
  
  return mjInitializing;
}

export async function renderMath(element: HTMLElement) {
  // Only init+render if the content actually has math
  const html = element.innerHTML;
  if (!containsMath(html)) return;
  
  await initMathJax();
  if (!(window as any).MathJax?.typesetPromise) return;
  try {
    await (window as any).MathJax.typesetPromise([element]);
  } catch (e) {
    console.warn('MathJax render error:', e);
  }
}

export function clearMathJaxCache(element: HTMLElement) {
  if (!(window as any).MathJax?.typesetClear) return;
  (window as any).MathJax.typesetClear([element]);
}

// Preprocess Anki's legacy math delimiters to standard MathJax delimiters
export function preprocessAnkiMath(html: string): string {
  return html
    .replace(/\[latex\]([\s\S]*?)\[\/latex\]/g, '\\[$1\\]')
    .replace(/\[\$\]([\s\S]*?)\[\/\$\]/g, '\\($1\\)');
}
