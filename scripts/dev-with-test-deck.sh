#!/usr/bin/env bash
# Start dev build and auto-seed a test deck via the frontend
export ANKIWRAPPER_SEED_TEST_DECK=true
npm run tauri dev
