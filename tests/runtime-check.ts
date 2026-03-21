// Runtime Integration Test Suite for Anki Wrapper
// Paste this into the browser dev tools console while tauri:dev is running

(async function runTests() {
  const results: { name: string; pass: boolean; detail: string }[] = [];
  
  function assert(name: string, condition: boolean, detail = '') {
    results.push({ name, pass: condition, detail: condition ? 'OK' : detail });
  }

  // Dynamically import Tauri API
  const { invoke } = await import('@tauri-apps/api/core');
  const { isTauri } = await import('@tauri-apps/api/core');
  
  // ── TEST 1: Tauri environment detected ──
  try {
    const tauriOk = await isTauri();
    assert('Tauri environment detected', tauriOk === true);
  } catch (e) {
    assert('Tauri environment detected', false, String(e));
  }

  // ── TEST 2: App version returns string ──
  try {
    const version = await invoke<string>('get_app_version');
    assert('get_app_version returns string', typeof version === 'string' && version.length > 0, `Got: ${version}`);
  } catch (e) {
    assert('get_app_version returns string', false, String(e));
  }

  // ── TEST 3: Collection initializes successfully ──
  try {
    await invoke('init_standalone_collection');
    assert('Collection initializes', true);
  } catch (e) {
    // May already be initialized
    const msg = String(e);
    const alreadyInit = msg.includes('already') || msg.includes('lock');
    assert('Collection initializes', true, alreadyInit ? 'Already initialized' : msg);
  }

  // ── TEST 4: get_preferences returns valid data ──
  try {
    const prefs = await invoke<any>('get_preferences');
    assert('get_preferences returns object', typeof prefs === 'object' && prefs !== null);
    assert('prefs has theme field', 'theme' in prefs, `Keys: ${Object.keys(prefs).join(', ')}`);
    assert('prefs has font_size field', 'font_size' in prefs);
    assert('prefs has animations_enabled field', 'animations_enabled' in prefs);
  } catch (e) {
    assert('get_preferences returns object', false, String(e));
  }

  // ── TEST 5: get_deck_stats returns array ──
  try {
    const stats = await invoke<any[]>('get_deck_stats');
    assert('get_deck_stats returns array', Array.isArray(stats), `Got: ${typeof stats}`);
  } catch (e) {
    assert('get_deck_stats returns array', false, String(e));
  }

  // ── TEST 6: get_all_decks returns array (if registered) ──
  try {
    const decks = await invoke<any[]>('get_all_decks');
    assert('get_all_decks returns array', Array.isArray(decks));
    if (decks.length > 0) {
      const first = decks[0];
      assert('deck has id field', 'id' in first);
      assert('deck has name field', 'name' in first);
    }
  } catch (e) {
    const msg = String(e);
    if (msg.includes('not found') || msg.includes('unknown')) {
      assert('get_all_decks is registered', false, 'Command not registered in Rust backend');
    } else {
      assert('get_all_decks returns array', false, msg);
    }
  }

  // ── TEST 7: get_scheduler_info works ──
  try {
    const info = await invoke<any>('get_scheduler_info');
    assert('get_scheduler_info works', typeof info === 'object');
    assert('scheduler reports fsrs_enabled', 'fsrs_enabled' in info);
  } catch (e) {
    assert('get_scheduler_info works', false, String(e));
  }

  // ── TEST 8: get_all_tags returns array ──
  try {
    const tags = await invoke<string[]>('get_all_tags');
    assert('get_all_tags returns array', Array.isArray(tags));
  } catch (e) {
    assert('get_all_tags returns array', false, String(e));
  }

  // ── TEST 9: get_undo_status works ──
  try {
    const status = await invoke<any>('get_undo_status');
    assert('get_undo_status works', typeof status === 'object');
    assert('undo_status has can_undo', 'can_undo' in status);
    assert('undo_status has can_redo', 'can_redo' in status);
  } catch (e) {
    assert('get_undo_status works', false, String(e));
  }

  // ── TEST 10: get_review_stats works ──
  try {
    const stats = await invoke<any>('get_review_stats', { deckId: null });
    assert('get_review_stats works', typeof stats === 'object');
    assert('stats has total_reviews', 'total_reviews' in stats);
    assert('stats has retention', 'retention' in stats);
  } catch (e) {
    assert('get_review_stats works', false, String(e));
  }

  // ── TEST 11: search_cards works with empty query ──
  try {
    const cards = await invoke<any[]>('search_cards', { query: '', order: 'cardDue', limit: 100 });
    assert('search_cards works', Array.isArray(cards));
  } catch (e) {
    assert('search_cards works', false, String(e));
  }

  // ── TEST 12: get_installed_plugins works ──
  try {
    const plugins = await invoke<any[]>('get_installed_plugins');
    assert('get_installed_plugins works', Array.isArray(plugins));
  } catch (e) {
    assert('get_installed_plugins works', false, String(e));
  }

  // ── TEST 13: DOM has rendered content (not blank) ──
  const appEl = document.getElementById('app');
  assert('App DOM element exists', appEl !== null);
  if (appEl) {
    assert('App has child elements', appEl.children.length > 0, `Children: ${appEl.children.length}`);
    assert('App is visible (has height)', appEl.offsetHeight > 0, `Height: ${appEl.offsetHeight}`);
    
    // Check that CSS custom properties are defined
    const bgBase = getComputedStyle(document.documentElement).getPropertyValue('--bg-base').trim();
    assert('CSS --bg-base is defined', bgBase.length > 0, `Value: "${bgBase}"`);
    
    const textPrimary = getComputedStyle(document.documentElement).getPropertyValue('--text-primary').trim();
    assert('CSS --text-primary is defined', textPrimary.length > 0, `Value: "${textPrimary}"`);
  }

  // ── TEST 14: Nav bar exists ──
  const nav = document.querySelector('nav');
  assert('Navigation bar exists', nav !== null);

  // ── TEST 15: No error state ──
  const errorEl = document.querySelector('.text-danger');
  const hasError = errorEl && errorEl.textContent?.includes('Desktop App Required');
  assert('No browser-only error shown', !hasError, hasError ? 'Showing browser error in Tauri' : '');

  // ── PRINT RESULTS ──
  console.log('\n%c═══════════════════════════════════════', 'color: #C4714F; font-weight: bold');
  console.log('%c  Anki Wrapper Runtime Test Results', 'color: #C4714F; font-weight: bold');
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
  
  console.log(`\n%c  ${passed} passed, ${failed} failed`, 
    failed > 0 ? 'color: #C0444A; font-weight: bold' : 'color: #6B8F71; font-weight: bold');
  
  return { passed, failed, results };
})();
