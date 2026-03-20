// Furigana Example Plugin
// Converts {漢字|かんじ} syntax to <ruby>漢字<rp>(</rp><rt>かんじ</rt><rp>)</rp></ruby>
//
// Usage: In your card content, write {漢字|かんじ} and this plugin
// will render it with furigana annotations.

(function() {
  'use strict';

  function addFurigana(data) {
    // Match {kanji|reading} pattern
    // The regex handles multi-character kanji and readings
    const furiganaRegex = /\{([^|{}]+)\|([^|{}]+)\}/g;

    const newHtml = data.html.replace(furiganaRegex, function(match, kanji, reading) {
      return '<ruby>' + kanji + '<rp>(</rp><rt>' + reading + '</rt><rp>)</rp></ruby>';
    });

    return { ...data, html: newHtml };
  }

  // Register for both front and back rendering
  __ankiPlugins.registerFilter('card:render:front', addFurigana);
  __ankiPlugins.registerFilter('card:render:back', addFurigana);
})();
