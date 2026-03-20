#!/usr/bin/env bash
set -euo pipefail

PASS=0
FAIL=0
RESULTS=""

log_pass() { PASS=$((PASS+1)); RESULTS="$RESULTS\n  ✅ $1"; }
log_fail() { FAIL=$((FAIL+1)); RESULTS="$RESULTS\n  ❌ $1: $2"; }

echo "========================================="
echo "  Anki Wrapper Build Verification Tests"
echo "========================================="
echo ""

# ───────────────────────────────────────────
# TEST 1: No Svelte 5 runes in plain .ts files
# ───────────────────────────────────────────
echo "TEST 1: No runes in plain .ts files..."
RUNE_VIOLATIONS=$(grep -rn '\$derived\|\$state\|\$effect\|\$props' src/lib/*.ts 2>/dev/null | grep -v '.svelte.ts' || true)
if [ -z "$RUNE_VIOLATIONS" ]; then
  log_pass "No Svelte 5 runes found in plain .ts files"
else
  log_fail "Svelte 5 runes found in plain .ts files" "$RUNE_VIOLATIONS"
fi

# ───────────────────────────────────────────
# TEST 2: No wildcard transition in :root
# ───────────────────────────────────────────
echo "TEST 2: No wildcard transition in :root..."
# Check if * { transition } appears inside a :root block
if python3 -c "
import re
css = open('src/app.css').read()
# Find all :root blocks
root_blocks = re.findall(r':root\s*\{([^}]*(?:\{[^}]*\}[^}]*)*)\}', css)
for block in root_blocks:
    if re.search(r'\*\s*\{', block):
        exit(1)
exit(0)
" 2>/dev/null; then
  log_pass "No wildcard * selector inside :root"
else
  log_fail "Wildcard * selector found inside :root" "This breaks CSS custom properties"
fi

# ───────────────────────────────────────────
# TEST 3: main.ts does NOT call prefs.load()
# ───────────────────────────────────────────
echo "TEST 3: main.ts has no premature prefs.load()..."
if grep -q 'prefs.load' src/main.ts 2>/dev/null; then
  log_fail "main.ts calls prefs.load()" "This runs invoke() before Tauri IPC is ready"
else
  log_pass "main.ts does not call prefs.load() at top level"
fi

# ───────────────────────────────────────────
# TEST 4: App.svelte does NOT use event-based init
# ───────────────────────────────────────────
echo "TEST 4: App.svelte uses sequential init (not event-based)..."
if grep -q 'collection:ready' src/App.svelte 2>/dev/null; then
  log_fail "App.svelte uses event-based collection:ready" "Rust backend never emits this event"
else
  log_pass "App.svelte does not use collection:ready event"
fi

# ───────────────────────────────────────────
# TEST 5: prefs.ts has all required fields
# ───────────────────────────────────────────
echo "TEST 5: prefs.ts has full preference fields..."
MISSING_FIELDS=""
for field in theme font_size daily_cutoff_hour show_remaining_count show_elapsed_time autoplay_audio show_intervals_on_buttons confirm_delete auto_backup backup_count; do
  if ! grep -q "$field" src/lib/prefs.ts 2>/dev/null; then
    MISSING_FIELDS="$MISSING_FIELDS $field"
  fi
done
if [ -z "$MISSING_FIELDS" ]; then
  log_pass "prefs.ts contains all required preference fields"
else
  log_fail "prefs.ts missing fields" "$MISSING_FIELDS"
fi

# ───────────────────────────────────────────
# TEST 6: toast.ts supports 'info' type
# ───────────────────────────────────────────
echo "TEST 6: toast.ts supports info type..."
if grep -q "'info'" src/lib/toast.ts 2>/dev/null; then
  log_pass "toast.ts supports 'info' toast type"
else
  log_fail "toast.ts missing 'info' type" "Some components call addToast with 'info'"
fi

# ───────────────────────────────────────────
# TEST 7: StudyView has interval fields on currentCard
# ───────────────────────────────────────────
echo "TEST 7: StudyView.svelte has interval fields..."
if grep -q 'again_interval' src/lib/StudyView.svelte 2>/dev/null; then
  log_pass "StudyView.svelte has interval fields on currentCard"
else
  log_fail "StudyView.svelte missing interval fields" "Answer buttons will show 'undefined'"
fi

# ───────────────────────────────────────────
# TEST 8: Frontend invoke calls match Rust handlers
# ───────────────────────────────────────────
echo "TEST 8: Frontend invoke calls have matching Rust handlers..."
# Extract frontend invoke targets
FRONTEND_CMDS=$(grep -roh 'invoke\(["'"'"'][a-z_]*["'"'"']' src/ 2>/dev/null | sed 's/invoke(["\x27]//;s/["\x27]//' | sort -u)
# Extract Rust handlers
RUST_CMDS=$(grep -A200 'invoke_handler' src-tauri/src/lib.rs 2>/dev/null | grep -oP '[a-z_]+' | sort -u)

MISSING_CMDS=""
for cmd in $FRONTEND_CMDS; do
  if ! echo "$RUST_CMDS" | grep -qx "$cmd"; then
    MISSING_CMDS="$MISSING_CMDS $cmd"
  fi
done

if [ -z "$MISSING_CMDS" ]; then
  log_pass "All frontend invoke targets have Rust handlers"
else
  log_fail "Missing Rust handlers for frontend calls" "$MISSING_CMDS"
fi

# ───────────────────────────────────────────
# TEST 9: Vite build succeeds
# ───────────────────────────────────────────
echo "TEST 9: Vite frontend build..."
if npm run build 2>&1 | tail -5; then
  log_pass "Vite build succeeded"
else
  log_fail "Vite build failed" "Check output above"
fi

# ───────────────────────────────────────────
# TEST 10: Svelte type checking
# ───────────────────────────────────────────
echo "TEST 10: Svelte type check..."
if npx svelte-check --tsconfig ./tsconfig.app.json 2>&1 | tail -10; then
  ERRORS=$(npx svelte-check --tsconfig ./tsconfig.app.json 2>&1 | grep -c 'Error' || true)
  if [ "$ERRORS" -eq 0 ]; then
    log_pass "Svelte type check passed with zero errors"
  else
    log_fail "Svelte type check found $ERRORS errors" "Run npx svelte-check for details"
  fi
else
  log_fail "Svelte type check failed to run" "Check configuration"
fi

# ───────────────────────────────────────────
# TEST 11: Rust compilation check
# ───────────────────────────────────────────
echo "TEST 11: Rust cargo check..."
cd src-tauri
if cargo check 2>&1 | tail -5; then
  log_pass "Rust cargo check passed"
else
  log_fail "Rust cargo check failed" "Check output above"
fi
cd ..

# ───────────────────────────────────────────
# TEST 12: No duplicate import patterns in App.svelte
# ───────────────────────────────────────────
echo "TEST 12: No duplicate imports in App.svelte..."
DUPES=$(grep '^  import' src/App.svelte 2>/dev/null | sort | uniq -d)
if [ -z "$DUPES" ]; then
  log_pass "No duplicate imports in App.svelte"
else
  log_fail "Duplicate imports in App.svelte" "$DUPES"
fi

# ───────────────────────────────────────────
# RESULTS SUMMARY
# ───────────────────────────────────────────
echo ""
echo "========================================="
echo "  RESULTS: $PASS passed, $FAIL failed"
echo "========================================="
printf "$RESULTS\n"
echo ""

if [ "$FAIL" -gt 0 ]; then
  echo "❌ SOME TESTS FAILED"
  exit 1
else
  echo "✅ ALL TESTS PASSED"
  exit 0
fi
