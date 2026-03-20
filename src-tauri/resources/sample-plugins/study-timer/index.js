(function() {
  'use strict';

  var sessionStart = null;
  var cardCount = 0;
  var ratingCounts = { 1: 0, 2: 0, 3: 0, 4: 0 };

  __ankiPlugins.registerAction('deck:study:start', function(data) {
    sessionStart = Date.now();
    cardCount = 0;
    ratingCounts = { 1: 0, 2: 0, 3: 0, 4: 0 };
    console.log('[Study Timer] Session started for deck:', data.deckName);
  });

  __ankiPlugins.registerAction('review:answer', function(data) {
    cardCount++;
    if (ratingCounts[data.rating] !== undefined) {
      ratingCounts[data.rating]++;
    }
  });

  __ankiPlugins.registerAction('deck:study:end', function(data) {
    if (!sessionStart) return;
    var elapsed = Math.round((Date.now() - sessionStart) / 1000);
    var minutes = Math.floor(elapsed / 60);
    var seconds = elapsed % 60;

    console.log('[Study Timer] Session complete:');
    console.log('  Duration:', minutes + 'm ' + seconds + 's');
    console.log('  Cards reviewed:', cardCount);
    console.log('  Again:', ratingCounts[1],
                '  Hard:', ratingCounts[2],
                '  Good:', ratingCounts[3],
                '  Easy:', ratingCounts[4]);

    sessionStart = null;
  });
})();
