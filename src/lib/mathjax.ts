// MathJax initialization and rendering utilities
// Matches Anki's default MathJax configuration

let mjInitialized = false;

export async function initMathJax() {
  if (mjInitialized) return;
  
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
  
  // Load MathJax components dynamically
  const MathJax = await import('mathjax/tex-svg.js');
  mjInitialized = true;
}

export async function renderMath(element: HTMLElement) {
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
