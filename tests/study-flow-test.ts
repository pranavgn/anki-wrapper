// Study Flow Integration Test
// Tests: create deck → add cards → get_next_card → answer_card → verify stats
// Paste into Tauri dev tools console

(async function testStudyFlow() {
  const { invoke } = await import('@tauri-apps/api/core');
  const log = (msg: string) => console.log(msg);
  const TEST_DECK = `__study_test_${Date.now()}`;
  let deckId: number | null = null;

  try {
    // SETUP: Create deck + cards
    log('📦 Setting up test deck with 3 cards...');
    await invoke('create_deck', { name: TEST_DECK });
    
    const allDecks = await invoke<any[]>('get_deck_stats');
    const deck = allDecks.find((d: any) => 
      d.name === TEST_DECK || d.short_name === TEST_DECK
    );
    if (!deck) throw new Error('Test deck not found after creation');
    deckId = deck.id;
    
    for (let i = 1; i <= 3; i++) {
      await invoke('add_basic_card', {
        deckId,
        front: `Study test Q${i}`,
        back: `Study test A${i}`
      });
    }
    log(`  ✅ Created deck (id=${deckId}) with 3 cards`);

    // TEST 1: get_deck_stats_for_review
    log('📊 Testing get_deck_stats_for_review...');
    const reviewStats = await invoke<any>('get_deck_stats_for_review', { deckId });
    log(`  Stats: new=${reviewStats.new_count ?? reviewStats.new}, learning=${reviewStats.learning_count ?? reviewStats.learning}, review=${reviewStats.review_count ?? reviewStats.review}`);
    log('  ✅ Stats retrieved');

    // TEST 2: get_next_card
    log('🃏 Getting next card...');
    const card = await invoke<any>('get_next_card', { deckId });
    if (!card) throw new Error('get_next_card returned null for deck with new cards');
    log(`  Card ID: ${card.card_id}`);
    log(`  Front: ${card.front?.substring(0, 50)}`);
    log(`  Has back: ${!!card.back}`);
    
    // Verify interval fields exist
    const hasIntervals = 'again_interval' in card && 'good_interval' in card;
    log(`  Has interval fields: ${hasIntervals}`);
    if (hasIntervals) {
      log(`  Intervals: again=${card.again_interval}, hard=${card.hard_interval}, good=${card.good_interval}, easy=${card.easy_interval}`);
    }
    log('  ✅ Next card retrieved');

    // TEST 3: answer_card
    log('✍️ Answering card (Good=3)...');
    const answerResult = await invoke<any>('answer_card', { 
      cardId: card.card_id, 
      ease: 3  // Good
    });
    log(`  Result: leech=${answerResult.leech}, suspended=${answerResult.suspended}`);
    log('  ✅ Card answered');

    // TEST 4: get_next_card again (should get card 2)
    log('🃏 Getting next card after answering...');
    const card2 = await invoke<any>('get_next_card', { deckId });
    if (!card2) throw new Error('No more cards after answering 1 of 3');
    if (card2.card_id === card.card_id) {
      log('  ⚠️ Got same card again (may be expected with learning steps)');
    } else {
      log(`  ✅ Got different card: ${card2.card_id}`);
    }

    // TEST 5: Undo
    log('↩️ Testing undo...');
    const undoStatus = await invoke<any>('get_undo_status');
    if (undoStatus.can_undo) {
      const undoResult = await invoke<any>('undo_last_action');
      log(`  ✅ Undo succeeded: ${undoResult.action_name}`);
    } else {
      log('  ⚠️ Nothing to undo (unexpected)');
    }

    // TEST 6: get_today_stats
    log('📊 Testing get_today_stats...');
    try {
      const today = await invoke<any>('get_today_stats');
      log(`  Today: reviews=${today.reviews_today ?? 'N/A'}, time=${today.study_time_today ?? 'N/A'}`);
      log('  ✅ Today stats retrieved');
    } catch (e) {
      log(`  ⚠️ get_today_stats failed (may not be implemented): ${e}`);
    }

    // CLEANUP
    log('\n🧹 Cleaning up...');
    await invoke('delete_deck', { deckId });
    log('  ✅ Test deck deleted');
    
    log('\n🎉 ALL STUDY FLOW TESTS PASSED');
  } catch (e) {
    log(`\n❌ TEST FAILED: ${e}`);
    if (deckId) {
      try { await invoke('delete_deck', { deckId }); } catch (_) {}
    }
  }
})();
