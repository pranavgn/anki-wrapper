// UI Rendering Smoke Test
// Verifies the blank screen is fixed and key UI elements are present
// Paste into Tauri dev tools console

(function testUIRendering() {
  const results: { name: string; pass: boolean; detail: string }[] = [];
  
  function check(name: string, condition: boolean, detail = '') {
    results.push({ name, pass: condition, detail: condition ? '✓' : detail || 'FAILED' });
  }

  // ── CSS Variables ──
  const root = getComputedStyle(document.documentElement);
  const bgBase = root.getPropertyValue('--bg-base').trim();
  check('CSS --bg-base defined', bgBase !== '' && bgBase !== 'undefined', `Value: "${bgBase}"`);
  check('CSS --bg-base is not black', bgBase !== '#000000' && bgBase !== '#000', `Value: "${bgBase}"`);
  
  const textPrimary = root.getPropertyValue('--text-primary').trim();
  check('CSS --text-primary defined', textPrimary !== '', `Value: "${textPrimary}"`);
  
  const accent = root.getPropertyValue('--accent').trim();
  check('CSS --accent defined', accent !== '', `Value: "${accent}"`);

  // ── App container ──
  const app = document.getElementById('app');
  check('App container exists', app !== null);
  check('App has children', (app?.children.length ?? 0) > 0, `${app?.children.length ?? 0} children`);
  check('App has visible height', (app?.offsetHeight ?? 0) > 50, `Height: ${app?.offsetHeight}px`);

  // ── Not showing browser error ──
  const browserError = document.querySelector('h2');
  const showingBrowserError = browserError?.textContent?.includes('Desktop App Required') ?? false;
  check('Not showing browser error', !showingBrowserError);

  // ── Navigation bar ──
  const nav = document.querySelector('nav');
  check('Navigation bar present', nav !== null);
  check('Nav has height', (nav?.offsetHeight ?? 0) > 20, `Height: ${nav?.offsetHeight}px`);
  
  // ── Nav buttons ──
  const navButtons = nav?.querySelectorAll('button') ?? [];
  check('Nav has buttons', navButtons.length >= 3, `Found ${navButtons.length} buttons`);
  
  // ── Main content area ──
  const main = document.querySelector('main');
  check('Main content area exists', main !== null);
  check('Main has visible content', (main?.offsetHeight ?? 0) > 100, `Height: ${main?.offsetHeight}px`);

  // ── Loading state ──
  const skeletons = document.querySelectorAll('.skeleton');
  const loadingText = document.body.textContent?.includes('loading') ?? false;
  // Either we have content loaded, or we're in loading state (both are acceptable, blank is not)
  const hasVisibleContent = (main?.textContent?.trim().length ?? 0) > 10;
  const isLoading = skeletons.length > 0;
  check('App has visible content or is loading', hasVisibleContent || isLoading, 
    `Content: ${hasVisibleContent}, Loading: ${isLoading}`);

  // ── Check no uncaught errors ──
  const errorOverlays = document.querySelectorAll('[data-error], .vite-error-overlay, #vite-error-overlay');
  check('No Vite error overlay', errorOverlays.length === 0);

  // ── Check body background is not white/transparent ──
  const bodyBg = getComputedStyle(document.body).backgroundColor;
  const isTransparent = bodyBg === 'rgba(0, 0, 0, 0)' || bodyBg === 'transparent';
  check('Body has background color', !isTransparent, `Background: ${bodyBg}`);

  // ── Print results ──
  console.log('\n%c═══════════════════════════════════════', 'color: #C4714F; font-weight: bold');
  console.log('%c  UI Rendering Smoke Test Results', 'color: #C4714F; font-weight: bold');
  console.log('%c═══════════════════════════════════════\n', 'color: #C4714F; font-weight: bold');
  
  let passed = 0, failed = 0;
  for (const r of results) {
    if (r.pass) {
      console.log(`%c  ✅ ${r.name}`, 'color: #6B8F71');
      passed++;
    } else {
      console.log(`%c  ❌ ${r.name}: ${r.detail}`, 'color: #C0444A');
      failed++;
    }
  }
  
  console.log(`\n  ${passed} passed, ${failed} failed`);
  return { passed, failed };
})();
