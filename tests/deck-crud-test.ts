// Deck CRUD Integration Test
// Paste into Tauri dev tools console

(async function testDeckCRUD() {
  const { invoke } = await import('@tauri-apps/api/core');
  const results: string[] = [];
  const log = (msg: string) => { results.push(msg); console.log(msg); };
  
  const TEST_DECK_NAME = `__test_deck_${Date.now()}`;
  let testDeckId: number | null = null;

  try {
    // 1. CREATE
    log('📝 Creating test deck...');
    await invoke('create_deck', { name: TEST_DECK_NAME });
    log('  ✅ Deck created');

    // 2. VERIFY EXISTS
    log('🔍 Verifying deck exists...');
    const decks = await invoke<any[]>('get_deck_stats');
    const found = decks.find((d: any) => d.name === TEST_DECK_NAME || d.short_name === TEST_DECK_NAME);
    if (!found) throw new Error('Created deck not found in get_deck_stats');
    testDeckId = found.id;
    log(`  ✅ Found deck with id=${testDeckId}`);

    // 3. ADD CARD
    log('📝 Adding test card...');
    await invoke('add_basic_card', { 
      deckId: testDeckId, 
      front: 'Test front', 
      back: 'Test back' 
    });
    log('  ✅ Card added');

    // 4. VERIFY CARD EXISTS
    log('🔍 Searching for card...');
    const cards = await invoke<any[]>('search_cards', { 
      query: `deck:"${TEST_DECK_NAME}"`, 
      sortOrder: 'cardDue' 
    });
    if (cards.length === 0) throw new Error('No cards found in test deck');
    log(`  ✅ Found ${cards.length} card(s)`);

    // 5. RENAME DECK
    const newName = TEST_DECK_NAME + '_renamed';
    log('✏️ Renaming deck...');
    await invoke('rename_deck', { deckId: testDeckId, newName });
    log('  ✅ Deck renamed');

    // 6. DELETE DECK
    log('🗑️ Deleting test deck...');
    await invoke('delete_deck', { deckId: testDeckId });
    log('  ✅ Deck deleted');

    // 7. VERIFY DELETED
    log('🔍 Verifying deletion...');
    const decksAfter = await invoke<any[]>('get_deck_stats');
    const stillExists = decksAfter.find((d: any) => d.id === testDeckId);
    if (stillExists) throw new Error('Deck still exists after deletion');
    log('  ✅ Deck confirmed deleted');

    log('\n🎉 ALL DECK CRUD TESTS PASSED');
  } catch (e) {
    log(`\n❌ TEST FAILED: ${e}`);
    
    // Cleanup: try to delete test deck if it exists
    if (testDeckId) {
      try {
        await invoke('delete_deck', { deckId: testDeckId });
        log('  🧹 Cleaned up test deck');
      } catch (_) {}
    }
  }
  
  return results;
})();
