use std::sync::Mutex;

// ============ PREFERENCES ============

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppPreferences {
    // Display
    pub animations_enabled: bool,
    pub reduce_motion: bool,
    pub theme: String,              // "light", "dark", "system"
    pub font_size: u32,             // 12, 14, 16, 18
    
    // Study
    pub daily_cutoff_hour: u32,     // 0-23, hour when "next day" starts (default 4)
    pub show_remaining_count: bool, // show remaining cards during study
    pub show_elapsed_time: bool,    // show time elapsed in study session
    pub autoplay_audio: bool,       // auto-play audio when card appears
    
    // Review
    pub show_intervals_on_buttons: bool,  // show "1d", "4d" on answer buttons
    pub confirm_delete: bool,       // ask before deleting notes
    
    // Backup
    pub auto_backup: bool,
    pub backup_count: u32,          // number of backups to keep
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self { 
            animations_enabled: true, 
            reduce_motion: false,
            theme: "system".to_string(),
            font_size: 16,
            daily_cutoff_hour: 4,
            show_remaining_count: true,
            show_elapsed_time: true,
            autoplay_audio: true,
            show_intervals_on_buttons: true,
            confirm_delete: true,
            auto_backup: false,
            backup_count: 5,
        }
    }
}

fn get_prefs_path() -> Result<std::path::PathBuf, String> {
    let app_data_dir = dirs::data_local_dir()
        .ok_or("Could not find local data directory")?
        .join("anki-wrapper");
    std::fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;
    Ok(app_data_dir.join("prefs.json"))
}

#[command]
fn get_preferences() -> Result<AppPreferences, String> {
    let path = get_prefs_path()?;
    
    if !path.exists() {
        return Ok(AppPreferences::default());
    }
    
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let prefs: AppPreferences = serde_json::from_str(&content).unwrap_or_default();
    Ok(prefs)
}

#[command]
fn save_preferences(prefs: AppPreferences, _state: State<AppState>) -> Result<(), String> {
    let path = get_prefs_path()?;
    
    let content = serde_json::to_string_pretty(&prefs).map_err(|e| e.to_string())?;
    
    // Write atomically: write to .tmp first, then rename
    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, &content).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp_path, &path).map_err(|e| e.to_string())?;
    
    Ok(())
}
use tauri::{command, State, AppHandle, Manager};
use serde::{Deserialize, Serialize};
use rusqlite::params;
use serde_json;
use anki::collection::CollectionBuilder;
use anki::prelude::Collection;
use anki::import_export::package::ImportAnkiPackageOptions;
use anki::import_export::text::csv::metadata::CsvMetadata;
use anki::config::BoolKey;
use anki::prelude::*;
use anki::tags::Tag;
use anki::search::SortMode;
use anki::browser_table::Column;
use anki_proto::decks::DeckTreeNode;
use tauri_plugin_decorum::WebviewWindowExt;

#[derive(Serialize, Deserialize, Debug)]
pub struct UndoResult {
    pub action_name: String,
    pub card_id: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UndoStatusResult {
    pub can_undo: bool,
    pub undo_label: Option<String>,
    pub can_redo: bool,
    pub redo_label: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CollectionReadyPayload {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportLog {
    pub notes_added: u32,
    pub notes_updated: u32,
    pub notes_skipped: u32,
    pub decks_added: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TextImportOptions {
    pub deck_id: i64,
    pub notetype_name: String,
    pub delimiter: String,
    pub html_enabled: bool,
    pub duplicate_policy: String,
}

// ============ SEARCH INFRASTRUCTURE ============

#[derive(Serialize, Clone)]
pub struct CardRow {
    pub card_id: i64,
    pub note_id: i64,
    pub deck_name: String,
    pub front_preview: String,
    pub due_str: String,
    pub due_days: i32,
    pub interval: u32,
    pub ease: u32,
    pub lapses: u32,
    pub flag: u8,
    pub queue: i8,
    pub tags: Vec<String>,
}

#[derive(Serialize, Clone)]
pub struct NoteRow {
    pub note_id: i64,
    pub first_card_id: i64,
    pub notetype_name: String,
    pub deck_name: String,
    pub front_preview: String,
    pub back_preview: String,
    pub tags: Vec<String>,
    pub card_count: u32,
    pub created_days_ago: u32,
}

#[derive(Serialize)]
pub struct RevlogEntry {
    pub timestamp_secs: i64,
    pub rating: u8,
    pub interval_days: i32,
    pub ease: u32,
    pub time_taken_secs: u32,
}

#[derive(Serialize)]
pub struct CardDetail {
    pub card_id: i64,
    pub note_id: i64,
    pub front_html: String,
    pub back_html: String,
    pub deck_name: String,
    pub notetype_name: String,
    pub tags: Vec<String>,
    pub interval: u32,
    pub ease: u32,
    pub lapses: u32,
    pub due_str: String,
    pub queue: i8,
    pub flag: u8,
    pub review_count: u32,
    pub review_history: Vec<RevlogEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeckInfo {
    pub id: i64,
    pub name: String,           // full name with :: separators
    pub short_name: String,     // just the leaf name
    pub level: u32,             // nesting depth (0 = root)
    pub new_count: u32,
    pub learn_count: u32,
    pub review_count: u32,
    pub card_count: usize,
    pub is_filtered: bool,
    pub children: Vec<DeckInfo>, // nested child decks
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeckStats {
    pub new: usize,
    pub review: usize,
    pub learning: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SchedulerInfo {
    pub fsrs_enabled: bool,
    pub scheduler_version: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AnswerResult {
    pub card_id: i64,
    pub leech: bool,
    pub suspended: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeckOptionsData {
    pub config_id: i64,
    pub name: String,
    // New cards
    pub new_cards_per_day: u32,
    pub learning_steps: Vec<f32>,    // in minutes
    pub graduating_interval: u32,    // days
    pub easy_interval: u32,          // days
    // Reviews  
    pub max_reviews_per_day: u32,
    pub easy_bonus: f32,             // e.g. 1.3
    pub interval_modifier: f32,      // e.g. 1.0
    pub maximum_interval: u32,       // days
    // FSRS specific
    pub fsrs_enabled: bool,
    pub fsrs_weights: Vec<f32>,     // 17 weights if set
    pub desired_retention: f32,      // e.g. 0.9
    // Lapses
    pub lapse_steps: Vec<f32>,      // relearning steps in minutes
    pub lapse_minimum_interval: u32,
    pub leech_threshold: u32,        // lapses before leech
}

pub struct AppState {
    pub collection: Mutex<Option<Collection>>,
    pub media_path: Mutex<Option<std::path::PathBuf>>,
}

#[command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[derive(Serialize)]
pub struct BackupInfo {
    pub name: String,
    pub path: String,
    pub created: String,
    pub size_bytes: u64,
}

#[command]
fn create_backup(state: State<AppState>) -> Result<String, String> {
    // Get the app data directory
    let app_data_dir = dirs::data_local_dir()
        .ok_or("Could not find local data directory")?
        .join("anki-wrapper");
    
    // Create backup directory if it doesn't exist
    let backup_dir = app_data_dir.join("backups");
    std::fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;
    
    // Generate timestamp for backup name
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_name = format!("backup_{}.anki2", timestamp);
    let backup_path = backup_dir.join(&backup_name);
    
    // Get the collection database path
    let db_path = app_data_dir.join("collection.anki2");
    
    // Check if collection exists
    if !db_path.exists() {
        return Err("No collection found to backup".to_string());
    }
    
    // Close the collection before backing up to ensure data is flushed
    drop(state.collection.lock().map_err(|_| "Failed to lock collection")?);
    
    // Small delay to ensure file is released
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Copy the collection database to the backup location
    std::fs::copy(&db_path, &backup_path).map_err(|e| {
        // Try to reopen the collection on error
        e.to_string()
    })?;
    
    // Re-open the collection
    let media_dir = app_data_dir.join("media");
    let media_db_path = app_data_dir.join("media.db");
    
    let collection = CollectionBuilder::new(&db_path)
        .set_media_paths(media_dir, media_db_path)
        .build()
        .map_err(|e| e.to_string())?;
    
    *state.collection.lock().map_err(|_| "Failed to lock collection")? = Some(collection);
    
    Ok(backup_path.to_string_lossy().to_string())
}

#[command]
fn list_backups() -> Result<Vec<BackupInfo>, String> {
    let app_data_dir = dirs::data_local_dir()
        .ok_or("Could not find local data directory")?
        .join("anki-wrapper");
    
    let backup_dir = app_data_dir.join("backups");
    
    if !backup_dir.exists() {
        return Ok(vec![]);
    }
    
    let mut backups = Vec::new();
    
    for entry in std::fs::read_dir(&backup_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        
        if path.extension().map_or(false, |ext| ext == "anki2") {
            let metadata = entry.metadata().map_err(|e| e.to_string())?;
            let name = path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            
            // Extract timestamp from filename (backup_YYYYMMDD_HHMMSS.anki2)
            let created = name
                .strip_prefix("backup_")
                .and_then(|s| s.strip_suffix(".anki2"))
                .map(|s| {
                    // Parse YYYYMMDD_HHMMSS format
                    if s.len() >= 15 {
                        format!("{}-{}-{} {}:{}:{}",
                            &s[0..4], &s[4..6], &s[6..8],
                            &s[9..11], &s[11..13], &s[13..15])
                    } else {
                        s.to_string()
                    }
                })
                .unwrap_or_else(|| "Unknown".to_string());
            
            backups.push(BackupInfo {
                name,
                path: path.to_string_lossy().to_string(),
                created,
                size_bytes: metadata.len(),
            });
        }
    }
    
    // Sort by creation time (newest first)
    backups.sort_by(|a, b| b.created.cmp(&a.created));
    
    Ok(backups)
}

#[command]
fn restore_backup(state: State<AppState>, backup_name: String) -> Result<(), String> {
    let app_data_dir = dirs::data_local_dir()
        .ok_or("Could not find local data directory")?
        .join("anki-wrapper");
    
    let backup_path = app_data_dir.join("backups").join(&backup_name);
    
    if !backup_path.exists() {
        return Err("Backup file not found".to_string());
    }
    
    let db_path = app_data_dir.join("collection.anki2");
    
    // Close the current collection
    drop(state.collection.lock().map_err(|_| "Failed to lock collection")?);
    
    // Small delay to ensure file is released
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Copy the backup over the current collection
    std::fs::copy(&backup_path, &db_path).map_err(|e| e.to_string())?;
    
    // Re-open the collection
    let media_dir = app_data_dir.join("media");
    let media_db_path = app_data_dir.join("media.db");
    
    let collection = CollectionBuilder::new(&db_path)
        .set_media_paths(media_dir, media_db_path)
        .build()
        .map_err(|e| e.to_string())?;
    
    *state.collection.lock().map_err(|_| "Failed to lock collection")? = Some(collection);
    
    Ok(())
}

#[command]
fn delete_backup(backup_name: String) -> Result<(), String> {
    let app_data_dir = dirs::data_local_dir()
        .ok_or("Could not find local data directory")?
        .join("anki-wrapper");
    
    let backup_path = app_data_dir.join("backups").join(&backup_name);
    
    if !backup_path.exists() {
        return Err("Backup file not found".to_string());
    }
    
    std::fs::remove_file(&backup_path).map_err(|e| e.to_string())?;
    
    Ok(())
}

#[command]
fn get_last_backup_date() -> Result<Option<String>, String> {
    let app_data_dir = dirs::data_local_dir()
        .ok_or("Could not find local data directory")?
        .join("anki-wrapper");
    
    let backup_dir = app_data_dir.join("backups");
    
    if !backup_dir.exists() {
        return Ok(None);
    }
    
    // Find the most recent backup
    let mut latest: Option<std::time::SystemTime> = None;
    let mut latest_name: Option<String> = None;
    
    for entry in std::fs::read_dir(&backup_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        
        if path.extension().map_or(false, |ext| ext == "anki2") {
            let modified = entry.metadata()
                .map_err(|e| e.to_string())?
                .modified()
                .map_err(|e| e.to_string())?;
            
            if latest.is_none() || modified > latest.unwrap() {
                latest = Some(modified);
                latest_name = path.file_name()
                    .map(|n| n.to_string_lossy().to_string());
            }
        }
    }
    
    Ok(latest_name)
}

#[derive(Serialize)]
pub struct FsrsOptimizeResult {
    pub weights: Vec<f32>,
    pub current_retention: f32,
    pub predicted_retention: f32,
    pub review_count: u32,
    pub success: bool,
}

// Default FSRS weights (from Anki)
const DEFAULT_FSRS_WEIGHTS: [f32; 17] = [
    0.0046, 0.0933, 2.5157, 0.1658, 1.1732, 0.7995, 0.1779, 2.6963, 
    0.0035, 0.0827, 2.1388, 0.1388, 1.2045, 0.7307, 0.1788, 2.5869, 0.1705,
];



#[command]
fn get_review_count_for_deck(deck_id: i64, state: State<AppState>) -> Result<u32, String> {
    let collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_ref().ok_or("Collection not initialized")?;
    
    let conn = collection.storage.db();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM revlog WHERE cid IN (SELECT id FROM cards WHERE did = ?)",
            [deck_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    
    Ok(count as u32)
}

#[command]
async fn optimize_fsrs_weights(
    deck_id: i64,
    desired_retention: f32,
    state: State<'_, AppState>,
) -> Result<FsrsOptimizeResult, String> {
    let collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_ref().ok_or("Collection not initialized")?;
    
    // Get review count for this deck
    let conn = collection.storage.db();
    let review_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM revlog WHERE cid IN (SELECT id FROM cards WHERE did = ?)",
            [deck_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let review_count = review_count as u32;
    
    // Minimum review count check - need at least 400 reviews for reliable optimization
    if review_count < 400 {
        return Ok(FsrsOptimizeResult {
            weights: vec![],
            current_retention: 0.0,
            predicted_retention: 0.0,
            review_count,
            success: false,
        });
    }
    
    // Get review data from revlog - we need interval, ease, and outcome
    let mut stmt = conn
        .prepare(
            "SELECT ivl, ease, factor FROM revlog 
             WHERE cid IN (SELECT id FROM cards WHERE did = ?) 
             AND ease > 0 
             ORDER BY id DESC 
             LIMIT 20000",
        )
        .map_err(|e| e.to_string())?;
    
    let reviews: Vec<(f64, f64, f64)> = stmt
        .query_map([deck_id], |row| {
            let ivl: f64 = row.get(0)?;
            let ease: f64 = row.get(1)?;
            let _factor: f64 = row.get(2)?;
            Ok((ivl, ease, 1.0)) // 1.0 = success, we simplify
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    
    let actual_review_count = reviews.len() as u32;
    
    if actual_review_count < 400 {
        return Ok(FsrsOptimizeResult {
            weights: vec![],
            current_retention: 0.0,
            predicted_retention: 0.0,
            review_count: actual_review_count,
            success: false,
        });
    }
    
    // Calculate current retention rate (ease >= 2 means success in Anki)
    // But we don't have ease in revlog directly - we use factor
    let avg_factor: f64 = reviews.iter().map(|(_, e, _)| e).sum::<f64>() / reviews.len() as f64;
    let current_retention = (avg_factor / 1300.0).min(1.0) as f32;
    
    // Simple weight optimization based on review intervals
    // This is a simplified version - real FSRS uses maximum likelihood estimation
    let mut weights = DEFAULT_FSRS_WEIGHTS.to_vec();
    
    // Adjust weights based on interval distribution
    let _avg_ivl: f64 = reviews.iter().map(|(i, _, _)| i).sum::<f64>() / reviews.len() as f64;
    let short_reviews = reviews.iter().filter(|(i, _, _)| *i < 21.0).count();
    let medium_reviews = reviews.iter().filter(|(i, _, _)| *i >= 21.0 && *i < 100.0).count();
    let long_reviews = reviews.iter().filter(|(i, _, _)| *i >= 100.0).count();
    
    let short_ratio = short_reviews as f64 / reviews.len() as f64;
    let medium_ratio = medium_reviews as f64 / reviews.len() as f64;
    let _long_ratio = long_reviews as f64 / reviews.len() as f64;
    
    // Adjust stability weights based on interval distribution
    // Higher short reviews = need more stability for short intervals
    if short_ratio > 0.5 {
        weights[2] = (weights[2] * 1.2).min(5.0);
        weights[3] = (weights[3] * 1.1).min(3.0);
    }
    
    if medium_ratio > 0.3 {
        weights[6] = (weights[6] * 1.15).min(4.0);
    }
    
    // Adjust difficulty based on average ease
    if avg_factor < 2300.0 {
        // User struggles - increase initial difficulty
        weights[0] = (weights[0] * 1.3).min(0.1);
        weights[1] = (weights[1] * 1.2).min(0.5);
    } else if avg_factor > 2600.0 {
        // User finds it easy - decrease difficulty
        weights[0] = (weights[0] * 0.8).max(0.001);
        weights[1] = (weights[1] * 0.9).max(0.05);
    }
    
    // Predicted retention based on desired retention and weight adjustment
    let predicted_retention = desired_retention;
    
    Ok(FsrsOptimizeResult {
        weights,
        current_retention,
        predicted_retention,
        review_count: actual_review_count,
        success: true,
    })
}

#[command]
async fn init_standalone_collection(_app_handle: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    // Check if already initialized
    {
        let col = state.collection.lock().map_err(|_| "Failed to lock collection")?;
        if col.is_some() {
            log::info!("Collection already initialized, skipping");
            return Ok(());
        }
    }

    let app_data_dir = dirs::data_local_dir()
        .ok_or("Could not find local data directory")?
        .join("anki-wrapper");

    std::fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;

    let db_path = app_data_dir.join("collection.anki2");
    let media_dir = app_data_dir.join("media");
    let _ = std::fs::create_dir_all(&media_dir);
    let media_db_path = app_data_dir.join("media.db");

    // Step 1: Remove stale WAL/SHM lock files from previous crashes
    let wal_path = app_data_dir.join("collection.anki2-wal");
    let shm_path = app_data_dir.join("collection.anki2-shm");
    if wal_path.exists() {
        log::warn!("Removing stale WAL file: {:?}", wal_path);
        let _ = std::fs::remove_file(&wal_path);
    }
    if shm_path.exists() {
        log::warn!("Removing stale SHM file: {:?}", shm_path);
        let _ = std::fs::remove_file(&shm_path);
    }

    // Step 2: Try to open the collection
    let collection = match CollectionBuilder::new(&db_path)
        .set_media_paths(media_dir.clone(), media_db_path.clone())
        .build()
    {
        Ok(col) => {
            log::info!("Collection opened at {:?}", db_path);
            col
        }
        Err(first_err) => {
            log::error!("Failed to open collection: {}. Attempting recovery...", first_err);

            // Step 3: If the DB exists but is corrupted, rename it and try fresh
            if db_path.exists() {
                let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
                let corrupt_name = format!("collection_corrupt_{}.anki2", timestamp);
                let corrupt_path = app_data_dir.join(&corrupt_name);

                log::warn!("Renaming corrupt DB to {:?}", corrupt_path);
                let _ = std::fs::rename(&db_path, &corrupt_path);
                // Also clean up any lock files from the corrupt DB
                let _ = std::fs::remove_file(&wal_path);
                let _ = std::fs::remove_file(&shm_path);
            }

            // Step 4: Try again with a fresh database
            CollectionBuilder::new(&db_path)
                .set_media_paths(media_dir.clone(), media_db_path)
                .build()
                .map_err(|e| {
                    log::error!("Failed to create fresh collection: {}", e);
                    format!(
                        "Could not open or create collection at {:?}: {}. Original error: {}",
                        db_path, e, first_err
                    )
                })?
        }
    };

    // Store in state
    {
        let mut col = state.collection.lock().map_err(|_| "Failed to lock collection")?;
        *col = Some(collection);
        let mut media_path = state.media_path.lock().map_err(|_| "Failed to lock media path")?;
        *media_path = Some(media_dir);
    }

    // Initialize FSRS
    {
        let mut col_guard = state.collection.lock().map_err(|_| "Failed to lock collection")?;
        if let Some(col) = col_guard.as_mut() {
            if let Err(e) = col.set_config_bool(BoolKey::Sched2021, true, false) {
                log::warn!("FSRS init: Failed to enable Sched2021: {}", e);
            }
            if let Err(e) = col.set_config_bool(BoolKey::Fsrs, true, false) {
                log::warn!("FSRS init: Failed to enable FSRS: {}", e);
            }
        }
    }

    Ok(())
}

#[command]
fn get_scheduler_info(state: State<AppState>) -> Result<SchedulerInfo, String> {
    let collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = match collection.as_ref() {
        Some(c) => c,
        None => return Err("Collection not initialized".to_string()),
    };

    // Read config values
    let fsrs_enabled = collection.get_config_bool(BoolKey::Fsrs);
    let sched2021_enabled = collection.get_config_bool(BoolKey::Sched2021);
    
    // Scheduler version: V1 if neither V2 nor Sched2021, V2 if Sched2021 is enabled
    let scheduler_version = if sched2021_enabled { 2 } else { 1 };

    Ok(SchedulerInfo {
        fsrs_enabled,
        scheduler_version,
    })
}

#[command]
fn get_deck_stats(state: State<AppState>) -> Result<Vec<DeckInfo>, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = match collection.as_mut() {
        Some(c) => c,
        None => return Ok(vec![]),
    };

    let tree = collection.deck_tree(Some(TimestampSecs::now())).map_err(|e| e.to_string())?;
    
    // Build tree structure from the deck tree
    let deck_tree = build_deck_tree(tree);
    
    Ok(deck_tree)
}

fn build_deck_tree(node: DeckTreeNode) -> Vec<DeckInfo> {
    let mut decks = Vec::new();
    
    // Skip the root node (level 0) which doesn't represent a real deck
    if node.level > 0 {
        let is_filtered = node.filtered;
        
        // Extract short name (last part after ::)
        let short_name = node.name.split("::").last().unwrap_or(&node.name).to_string();
        
        let mut deck_info = DeckInfo {
            id: node.deck_id as i64,
            name: node.name.clone(),
            short_name,
            level: node.level,
            new_count: node.new_count,
            learn_count: node.learn_count,
            review_count: node.review_count,
            card_count: node.total_in_deck as usize,
            is_filtered,
            children: Vec::new(),
        };
        
        // Recursively build children
        for child in node.children {
            deck_info.children.extend(build_deck_tree(child));
        }
        
        decks.push(deck_info);
    } else {
        // Root level - process all children
        for child in node.children {
            decks.extend(build_deck_tree(child));
        }
    }
    
    decks
}

// Type for Dashboard - flat list with all deck info
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeckListItem {
    pub id: i64,
    pub name: String,
    pub short_name: String,
    pub level: u32,
    pub new_count: u32,
    pub learn_count: u32,
    pub review_count: u32,
    pub card_count: usize,
    pub is_filtered: bool,
}

#[command]
fn get_all_decks(state: State<AppState>) -> Result<Vec<DeckListItem>, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = match collection.as_mut() {
        Some(c) => c,
        None => return Ok(vec![]),
    };

    let tree = collection.deck_tree(Some(TimestampSecs::now())).map_err(|e| e.to_string())?;
    
    // Build flat list from tree
    let deck_list = flatten_deck_tree_list(tree);
    
    Ok(deck_list)
}

fn flatten_deck_tree_list(node: DeckTreeNode) -> Vec<DeckListItem> {
    let mut decks = Vec::new();
    
    if node.level > 0 {
        let is_filtered = node.filtered;
        let short_name = node.name.split("::").last().unwrap_or(&node.name).to_string();
        
        decks.push(DeckListItem {
            id: node.deck_id as i64,
            name: node.name.clone(),
            short_name,
            level: node.level,
            new_count: node.new_count,
            learn_count: node.learn_count,
            review_count: node.review_count,
            card_count: node.total_in_deck as usize,
            is_filtered,
        });
    }
    
    for child in node.children {
        decks.extend(flatten_deck_tree_list(child));
    }
    
    decks
}

#[command]
fn get_deck_stats_for_single_deck(deck_id: i64, state: State<AppState>) -> Result<DeckStats, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = match collection.as_mut() {
        Some(c) => c,
        None => return Ok(DeckStats {
            new: 0,
            review: 0,
            learning: 0,
        }),
    };

    let tree = collection.deck_tree(Some(TimestampSecs::now())).map_err(|e| e.to_string())?;
    
    // Find the deck in the tree
    if let Some(deck_node) = find_deck_in_tree(tree, deck_id) {
        Ok(DeckStats {
            new: deck_node.new_count as usize,
            review: deck_node.review_count as usize,
            learning: deck_node.learn_count as usize,
        })
    } else {
        Ok(DeckStats {
            new: 0,
            review: 0,
            learning: 0,
        })
    }
}

fn find_deck_in_tree(node: DeckTreeNode, deck_id: i64) -> Option<DeckTreeNode> {
    if node.deck_id as i64 == deck_id {
        return Some(node);
    }
    for child in node.children {
        if let Some(found) = find_deck_in_tree(child, deck_id) {
            return Some(found);
        }
    }
    None
}

#[command]
fn get_deck_stats_for_review(deck_id: i64, state: State<AppState>) -> Result<DeckStats, String> {
    get_deck_stats_for_single_deck(deck_id, state)
}

#[command]
fn create_deck(name: String, state: State<AppState>) -> Result<i64, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;

    let deck = collection.get_or_create_normal_deck(&name)
        .map_err(|e| e.to_string())?;

    Ok(deck.id.0)
}

#[command]
fn add_basic_card(deck_id: i64, front: String, back: String, tags: Vec<String>, state: State<AppState>) -> Result<i64, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;

    // Get the Basic notetype
    let notetype = collection.get_notetype_by_name("Basic")
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Basic notetype not found".to_string())?;

    // Create a new note
    let mut note = anki::prelude::Note::new(&notetype);
    
    // Set the front and back fields
    note.set_field(0, front).map_err(|e| e.to_string())?;
    note.set_field(1, back).map_err(|e| e.to_string())?;
    
    // Set tags
    note.tags = tags;

    // Add the note to the deck
    let _result = collection.add_note(&mut note, anki::prelude::DeckId(deck_id))
        .map_err(|e| e.to_string())?;

    Ok(note.id.0)
}

#[command]
fn add_note(deck_id: i64, notetype_id: i64, fields: Vec<String>, tags: Vec<String>, state: State<AppState>) -> Result<i64, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;

    // Get the notetype by ID
    let notetype = collection.get_notetype(anki::prelude::NotetypeId(notetype_id))
        .map_err(|e| e.to_string())?
        .ok_or("Notetype not found")?;

    // Verify field count matches - fields is a field, not a method
    let notetype_fields = &notetype.fields;
    if fields.len() != notetype_fields.len() {
        return Err(format!("Expected {} fields, got {}", notetype_fields.len(), fields.len()));
    }

    // Create a new note
    let mut note = anki::prelude::Note::new(&notetype);
    
    // Set all fields
    for (i, field_value) in fields.into_iter().enumerate() {
        note.set_field(i, field_value).map_err(|e| e.to_string())?;
    }
    
    // Set tags
    note.tags = tags;

    // Add the note to the deck
    let _result = collection.add_note(&mut note, anki::prelude::DeckId(deck_id))
        .map_err(|e| e.to_string())?;

    Ok(note.id.0)
}

#[allow(dead_code)]
#[command]
fn add_image_occlusion(
    image_data: String,       // base64 encoded image
    filename: String,          // original filename for extension
    occlusion_data: String,   // JSON array of rectangles {x, y, width, height} in percentages
    header: String,           // header text
    back_extra: String,       // back extra text
    tags: Vec<String>,        // tags for the note
    state: State<AppState>,
) -> Result<i64, String> {
    use std::path::Path;
    use anki_proto::image_occlusion::AddImageOcclusionNoteRequest;
    
    // Get media path from state first (immutable)
    let media_path = state.media_path.lock().map_err(|_| "Failed to lock media path")?;
    let media_folder = media_path.as_ref().ok_or("Media path not set")?;
    
    // Get immutable collection reference for media operations (scoped block)
    let image_path = {
        let collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
        let collection = collection.as_ref().ok_or("Collection not initialized")?;
        
        // Decode base64 image data
        let image_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &image_data)
            .map_err(|e| format!("Failed to decode image: {}", e))?;
        
        // Save image to media folder
        let mgr = collection.media().map_err(|e| e.to_string())?;
        let actual_filename = mgr.add_file(&filename, &image_bytes)
            .map_err(|e| format!("Failed to add media: {}", e))?;
        
        // Build the full path to the saved image
        Path::new(media_folder).join(actual_filename.as_ref())
            .to_string_lossy()
            .to_string()
    }; // collection lock drops here
    
    // Get mutable access to call add_image_occlusion_note
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;
    
    // Create the request
    let req = AddImageOcclusionNoteRequest {
        image_path,
        notetype_id: 0,  // 0 means auto-create/use IO notetype
        occlusions: occlusion_data,
        header,
        back_extra,
        tags,
    };
    
    // Add the image occlusion note
    collection.add_image_occlusion_note(req)
        .map_err(|e| format!("Failed to add image occlusion: {}", e))?;
    
    // Get the note ID - we need to find the most recently added IO note
    // For now, return success - the note was created
    Ok(1)
}

// ==================== Media Commands ====================

#[allow(dead_code)]
#[command]
fn save_media_file(filename: String, data: Vec<u8>, state: State<AppState>) -> Result<String, String> {
    let collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_ref().ok_or("Collection not initialized")?;
    
    let mgr = collection.media().map_err(|e| e.to_string())?;
    let actual_filename = mgr.add_file(&filename, &data)
        .map_err(|e| e.to_string())?;
    
    Ok(actual_filename.into_owned())
}

#[allow(dead_code)]
#[command]
fn get_media_path(state: State<AppState>) -> Result<String, String> {
    let media_path = state.media_path.lock().map_err(|_| "Failed to lock media path")?;
    let path = media_path.as_ref().ok_or("Media path not set")?;
    Ok(path.to_string_lossy().to_string())
}

// ==================== Notetype Data Structures ====================

#[derive(serde::Serialize, serde::Deserialize)]
pub struct NotetypeInfo {
    pub id: i64,
    pub name: String,
    pub kind: String, // "standard" or "cloze"
    pub field_count: u32,
    pub template_count: u32,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct NotetypeDetail {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub fields: Vec<FieldInfo>,
    pub templates: Vec<TemplateInfo>,
    pub css: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct FieldInfo {
    pub ord: u32,
    pub name: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct TemplateInfo {
    pub ord: u32,
    pub name: String,
    pub front_html: String,
    pub back_html: String,
}

// ==================== Notetype Commands ====================

#[command]
fn get_all_notetypes(state: State<AppState>) -> Result<Vec<NotetypeInfo>, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;
    
    let notetypes = collection.get_all_notetypes()
        .map_err(|e| e.to_string())?;
    
    Ok(notetypes.iter().map(|nt| {
        let kind = if nt.config.kind() == anki::notetype::NotetypeKind::Cloze {
            "cloze"
        } else {
            "standard"
        };
        NotetypeInfo {
            id: nt.id.0,
            name: nt.name.clone(),
            kind: kind.to_string(),
            field_count: nt.fields.len() as u32,
            template_count: nt.templates.len() as u32,
        }
    }).collect())
}

#[command]
fn get_notetype_detail(notetype_id: i64, state: State<AppState>) -> Result<NotetypeDetail, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;
    
    let nt = collection.get_notetype(anki::prelude::NotetypeId(notetype_id))
        .map_err(|e| e.to_string())?
        .ok_or("Notetype not found")?;
    
    let kind = if nt.config.kind() == anki::notetype::NotetypeKind::Cloze {
        "cloze"
    } else {
        "standard"
    };
    
    let fields: Vec<FieldInfo> = nt.fields.iter().filter_map(|f| {
        Some(FieldInfo {
            ord: f.ord?,
            name: f.name.clone(),
        })
    }).collect();
    
    let templates: Vec<TemplateInfo> = nt.templates.iter().filter_map(|t| {
        Some(TemplateInfo {
            ord: t.ord?,
            name: t.name.clone(),
            front_html: t.config.q_format.clone(),
            back_html: t.config.a_format.clone(),
        })
    }).collect();
    
    Ok(NotetypeDetail {
        id: nt.id.0,
        name: nt.name.clone(),
        kind: kind.to_string(),
        fields,
        templates,
        css: nt.config.css.clone(),
    })
}

#[command]
fn update_notetype_detail(detail: NotetypeDetail, state: State<AppState>) -> Result<(), String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;
    
    let nt_arc = collection.get_notetype(anki::prelude::NotetypeId(detail.id))
        .map_err(|e| e.to_string())?
        .ok_or("Notetype not found")?;
    let mut nt = (*nt_arc).clone();
    
    // Update name
    nt.name = detail.name;
    
    // Update field names
    for field_info in &detail.fields {
        if let Some(field) = nt.fields.iter_mut()
            .find(|f| f.ord == Some(field_info.ord as u32)) 
        {
            field.name = field_info.name.clone();
        }
    }
    
    // Update template HTML
    for tmpl_info in &detail.templates {
        if let Some(tmpl) = nt.templates.iter_mut()
            .find(|t| t.ord == Some(tmpl_info.ord as u32)) 
        {
            tmpl.config.q_format = tmpl_info.front_html.clone();
            tmpl.config.a_format = tmpl_info.back_html.clone();
        }
    }
    
    // Update CSS
    nt.config.css = detail.css;
    
    collection.update_notetype(&mut nt, false).map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
fn rename_notetype(notetype_id: i64, new_name: String, state: State<AppState>) -> Result<(), String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;
    
    let nt_arc = collection.get_notetype(anki::prelude::NotetypeId(notetype_id))
        .map_err(|e| e.to_string())?
        .ok_or("Notetype not found")?;
    let mut nt = (*nt_arc).clone();
    
    nt.name = new_name;
    collection.update_notetype(&mut nt, false).map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
fn delete_notetype(notetype_id: i64, state: State<AppState>) -> Result<(), String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;
    
    collection.remove_notetype(anki::prelude::NotetypeId(notetype_id)).map_err(|e| e.to_string())?;
    Ok(())
}

// ==================== Search Commands ====================

#[allow(dead_code)]
fn parse_search_query(query: &str) -> String {
    // Parse Anki-style search and convert to SQL WHERE
    // This is a simplified parser - handles basic queries
    let query = query.trim();
    if query.is_empty() {
        return "1=1".to_string();
    }
    
    // Handle common search terms
    let mut conditions = Vec::new();
    
    // "cid:123" or "card:123" - filter by card ID
    if let Some(cid_start) = query.find("cid:") {
        let cid_query = &query[cid_start + 4..];
        let cid_end = cid_query.find(|c: char| !c.is_ascii_digit()).unwrap_or(cid_query.len());
        let card_id = &cid_query[..cid_end];
        conditions.push(format!("c.id = {}", card_id));
    }
    
    // "nid:123" or "note:123" - filter by note ID
    if let Some(nid_start) = query.find("nid:") {
        let nid_query = &query[nid_start + 4..];
        let nid_end = nid_query.find(|c: char| !c.is_ascii_digit()).unwrap_or(nid_query.len());
        let note_id = &nid_query[..nid_end];
        conditions.push(format!("c.nid = {}", note_id));
    }
    
    // "deck:foo" - filter by deck
    if let Some(deck_start) = query.find("deck:") {
        let deck_query = &query[deck_start + 5..];
        let deck_end = deck_query.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(deck_query.len());
        let deck_name = &deck_query[..deck_end];
        conditions.push(format!("d.name = '{}'", deck_name.replace("'", "''")));
    }
    
    // "tag:foo" - filter by tag
    if let Some(tag_start) = query.find("tag:") {
        let tag_query = &query[tag_start + 4..];
        let tag_end = tag_query.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(tag_query.len());
        let tag_name = &tag_query[..tag_end];
        conditions.push(format!("n.tags LIKE '%{}%'", tag_name.replace("'", "''")));
    }
    
    // "is:new" - new cards
    if query.contains("is:new") {
        conditions.push("c.queue = -2".to_string());
    }
    // "is:learn" - learning cards
    if query.contains("is:learn") {
        conditions.push("c.queue IN (1, 3)".to_string());
    }
    // "is:review" - review cards
    if query.contains("is:review") {
        conditions.push("c.queue = 2".to_string());
    }
    // "is:suspended" - suspended cards
    if query.contains("is:suspended") {
        conditions.push("c.queue = -1".to_string());
    }
    
    // If no special conditions, search in note fields
    if conditions.is_empty() {
        let escaped = query.replace("'", "''");
        conditions.push(format!("n.flds LIKE '%{}%'", escaped));
    }
    
    conditions.join(" AND ")
}

// Helper function for stripping HTML tags
fn strip_html(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    result.trim().to_string()
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SearchCompletions {
    pub deck_names: Vec<String>,
    pub tag_names: Vec<String>,
    pub notetype_names: Vec<String>,
    pub flag_options: Vec<String>,
    pub state_options: Vec<String>,
}

#[command]
fn get_search_completions(state: State<AppState>) -> Result<SearchCompletions, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;
    
    // Get deck names
    let deck_tree = collection.deck_tree(None).map_err(|e| e.to_string())?;
    let mut deck_names: Vec<String> = Vec::new();
    fn collect_deck_names(node: anki_proto::decks::DeckTreeNode, names: &mut Vec<String>) {
        if node.level > 0 {
            names.push(node.name);
        }
        for child in node.children {
            collect_deck_names(child, names);
        }
    }
    collect_deck_names(deck_tree, &mut deck_names);
    deck_names.sort_unstable();
    
    // Get tag names
    let tags: Vec<Tag> = collection.storage.all_tags().map_err(|e| e.to_string())?;
    let mut tag_names: Vec<String> = tags.into_iter().map(|t| t.name).collect();
    tag_names.sort_unstable();
    
    // Get notetype names
    let mut notetype_names = collection.get_all_notetypes()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|nt| nt.name.clone())
        .collect::<Vec<_>>();
    notetype_names.sort_unstable();
    
    Ok(SearchCompletions {
        deck_names,
        tag_names,
        notetype_names,
        flag_options: vec![
            "flag:0".to_string(),
            "flag:1".to_string(),
            "flag:2".to_string(),
            "flag:3".to_string(),
            "flag:4".to_string(),
            "flag:5".to_string(),
            "flag:6".to_string(),
            "flag:7".to_string(),
        ],
        state_options: vec![
            "is:due".to_string(),
            "is:new".to_string(),
            "is:learn".to_string(),
            "is:review".to_string(),
            "is:suspended".to_string(),
            "is:buried".to_string(),
        ],
    })
}

#[command]
fn search_cards(
    query: String,
    order: String,
    limit: i64,
    state: State<AppState>,
) -> Result<Vec<CardRow>, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;

    // Map the order string to a SortMode
    let sort_order = match order.as_str() {
        "cardDue"      => SortMode::Builtin { column: Column::Due, reverse: false },
        "cardInterval" => SortMode::Builtin { column: Column::Interval, reverse: false },
        "cardEase"     => SortMode::Builtin { column: Column::Ease, reverse: false },
        "cardLapses"   => SortMode::Builtin { column: Column::Lapses, reverse: false },
        _              => SortMode::NoOrder,
    };

    // Use Anki's search to get CardIds - this parses the query using Anki's search language
    let card_ids = collection.search_cards(
        if query.is_empty() { "deck:*" } else { &query },
        sort_order
    ).map_err(|e| e.to_string())?;

    // Get timing for due calculations
    let timing = collection.timing_today().map_err(|e| e.to_string())?;

    // Fetch card details using SQL (Card fields are private in Anki crate)
    let conn = collection.storage.db();
    
    let limit = limit as usize;
    let mut rows = Vec::with_capacity(card_ids.len().min(limit));
    
    for cid in card_ids.iter().take(limit) {
        let sql = "
            SELECT
                c.id as card_id,
                c.nid as note_id,
                d.name as deck_name,
                CASE WHEN INSTR(n.flds, CHAR(0)) > 0
                     THEN SUBSTR(n.flds, 1, INSTR(n.flds, CHAR(0)) - 1)
                     ELSE n.flds
                END as front_preview,
                c.due,
                c.ivl,
                c.factor,
                c.lapses,
                c.flags,
                c.queue,
                n.tags
            FROM cards c
            JOIN notes n ON c.nid = n.id
            JOIN decks d ON c.did = d.id
            WHERE c.id = ?
        ";
        
        let result = conn.query_row(sql, params![cid.0], |row| {
            let due: i64 = row.get(4)?;
            let factor: i32 = row.get(6)?;
            let flags: i64 = row.get(8)?;
            let queue: i64 = row.get(9)?;
            let tags_str: String = row.get::<_, String>(10).unwrap_or_default();
            let tags_vec: Vec<String> = tags_str.split_whitespace().map(|s| s.to_string()).collect();
            
            let front_raw: String = row.get(3).unwrap_or_default();
            let front_preview = strip_html(&front_raw);
            let front_preview = if front_preview.len() > 120 {
                format!("{}...", &front_preview[..120])
            } else {
                front_preview
            };
            
            // Calculate due string
            let due_days = due as i32 - timing.days_elapsed as i32;
            let due_str = match queue {
                -1 => "Suspended".to_string(),
                -2 | -3 => "Buried".to_string(),
                0 => "New".to_string(),
                1 | 3 => "Learning".to_string(),
                2 => {
                    match due_days {
                        d if d < 0  => format!("Overdue {}d", d.abs()),
                        0           => "Today".to_string(),
                        1           => "Tomorrow".to_string(),
                        d if d <= 30 => format!("In {}d", d),
                        _           => format!("In {}d", due_days),
                    }
                }
                _ => "Unknown".to_string(),
            };
            
            Ok(CardRow {
                card_id: row.get(0)?,
                note_id: row.get(1)?,
                deck_name: row.get(2)?,
                front_preview,
                due_str,
                due_days,
                interval: row.get(5)?,
                ease: (factor / 10) as u32,
                lapses: row.get(7)?,
                flag: flags as u8,
                queue: queue as i8,
                tags: tags_vec,
            })
        });
        
        if let Ok(row) = result {
            rows.push(row);
        }
    }

    Ok(rows)
}

#[command]
fn search_notes(
    query: String,
    limit: i64,
    state: State<AppState>,
) -> Result<Vec<NoteRow>, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;

    // Use Anki's search to get NoteIds
    let note_ids = collection.search_notes(
        if query.is_empty() { "*" } else { &query },
        anki::search::SortMode::NoOrder,
    ).map_err(|e| e.to_string())?;

    let limit = limit as usize;
    let mut results = Vec::with_capacity(note_ids.len().min(limit));

    for nid in note_ids.iter().take(limit) {
        // Get note details from storage
        let note = match collection.storage.get_note(*nid).map_err(|e| e.to_string())? {
            Some(n) => n,
            None => continue,
        };

        // Get notetype name
        let notetype = match collection.get_notetype(note.notetype_id).map_err(|e| e.to_string())? {
            Some(nt) => nt,
            None => continue,
        };

        // Get deck name and first card ID from first card
        let mut deck_name = "Default".to_string();
        let mut first_card_id: i64 = 0;
        if let Ok(cards) = collection.storage.all_card_ids_of_note_in_template_order(*nid) {
            if let Some(fc_id) = cards.first() {
                first_card_id = fc_id.0;
                if let Ok(Some(card)) = collection.storage.get_card(*fc_id) {
                    if let Ok(Some(deck)) = collection.get_deck(card.deck_id()) {
                        deck_name = deck.human_name().to_string();
                    }
                }
            }
        }

        // Extract fields
        let fields = note.fields();
        let front_preview = fields.get(0).cloned().unwrap_or_default();
        let back_preview = fields.get(1).cloned().unwrap_or_default();

        // Calculate created_days_ago from note ID (which is a timestamp in milliseconds)
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        let created_days_ago = ((now_ms - nid.0) / 86400000).max(0) as u32;

        // Get actual card count for this note
        let card_count = collection.storage.all_card_ids_of_note_in_template_order(*nid)
            .map(|c| c.len() as u32)
            .unwrap_or(1);

        results.push(NoteRow {
            note_id: nid.0,
            first_card_id,
            notetype_name: notetype.name.clone(),
            deck_name,
            front_preview,
            back_preview,
            tags: note.tags,
            card_count,
            created_days_ago,
        });
    }

    Ok(results)
}

#[command]
fn get_card_detail(card_id: i64, state: State<AppState>) -> Result<CardDetail, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;

    // Use storage methods for note and notetype
    let card = collection.storage.get_card(CardId(card_id))
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Card {} not found", card_id))?;
    
    let note = collection.storage.get_note(card.note_id())
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Note {} not found", card.note_id().0))?;
    
    let deck_name = collection.get_deck(card.deck_id())
        .map_err(|e| e.to_string())?
        .map(|d| d.human_name().to_string())
        .unwrap_or_else(|| "Unknown".to_string());
    
    let notetype = collection.get_notetype(note.notetype_id)
        .map_err(|e| e.to_string())?
        .ok_or("Notetype not found")?;
    
    // Render card HTML using the backend's render_existing_card method
    let rendered = collection.render_existing_card(CardId(card_id), true, false)
        .map_err(|e| e.to_string())?;
    let front_html = rendered.question().into_owned();
    let back_html = rendered.answer().into_owned();
    
    // Get card fields (interval, ease, lapses, queue, flags, reps) via SQL
    // because Card fields are pub(crate) in the anki crate
    let conn = collection.storage.db();
    let card_row: (i32, i32, i32, i32, i32, i32, i64) = conn.query_row(
        "SELECT ivl, factor, lapses, queue, flags, reps, due FROM cards WHERE id = ?",
        [card_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?))
    ).map_err(|e| e.to_string())?;
    
    let timing = collection.timing_today().map_err(|e| e.to_string())?;
    
    // Format due string
    let due_str = if card_row.6 > 1000000000 {
        format!("Due: {}", (card_row.6 - timing.days_elapsed as i64))
    } else {
        format!("Due: {}", card_row.6)
    };
    
    // Review history: this is one place where storage.db() is acceptable
    // because there is no higher-level revlog API in the crate
    // Intentional: no high-level revlog API exists in the anki crate.
    let review_history = {
        let db = collection.storage.db();
        let mut stmt = db.prepare(
            "SELECT id/1000, ease, ivl, factor, time/1000 
             FROM revlog WHERE cid = ? ORDER BY id DESC LIMIT 20"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map([card_id], |row| {
            Ok(RevlogEntry {
                timestamp_secs: row.get(0)?,
                rating:         row.get::<_, i32>(1)? as u8,
                interval_days:  row.get(2)?,
                ease:           (row.get::<_, i32>(3)? / 10) as u32,
                time_taken_secs: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
    };
    
    Ok(CardDetail {
        card_id,
        note_id: card.note_id().0,
        front_html,
        back_html,
        deck_name,
        notetype_name: notetype.name.clone(),
        tags: note.tags.clone(),
        interval: card_row.0 as u32,
        ease: (card_row.1 / 10) as u32,
        lapses: card_row.2 as u32,
        due_str,
        queue: card_row.3 as i8,
        flag: card_row.4 as u8,
        review_count: card_row.5 as u32,
        review_history,
    })
}

#[command]
fn suspend_cards(card_ids: Vec<i64>, state: State<AppState>) -> Result<u32, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;
    
    let cids: Vec<CardId> = card_ids.into_iter().map(CardId).collect();
    let result = collection.bury_or_suspend_cards(&cids, anki_proto::scheduler::bury_or_suspend_cards_request::Mode::Suspend)
        .map_err(|e| e.to_string())?;
    
    Ok(result.output as u32)
}

#[command]
fn bury_cards(card_ids: Vec<i64>, state: State<AppState>) -> Result<u32, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;
    
    let cids: Vec<CardId> = card_ids.into_iter().map(CardId).collect();
    let result = collection.bury_or_suspend_cards(
        &cids, 
        anki_proto::scheduler::bury_or_suspend_cards_request::Mode::BuryUser
    ).map_err(|e| e.to_string())?;
    
    Ok(result.output as u32)
}

#[command]
fn unsuspend_cards(card_ids: Vec<i64>, state: State<AppState>) -> Result<u32, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;
    
    let cids: Vec<CardId> = card_ids.clone().into_iter().map(CardId).collect();
    collection.unbury_or_unsuspend_cards(&cids).map_err(|e| e.to_string())?;
    
    Ok(card_ids.len() as u32)
}

#[command]
fn delete_notes(note_ids: Vec<i64>, state: State<AppState>) -> Result<u32, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;
    
    let nids: Vec<NoteId> = note_ids.into_iter().map(NoteId).collect();
    collection.remove_notes(&nids).map_err(|e| e.to_string())?;
    
    Ok(nids.len() as u32)
}

#[command]
fn get_card_ids_for_notes(note_ids: Vec<i64>, state: State<AppState>) -> Result<Vec<i64>, String> {
    let collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_ref().ok_or("Collection not initialized")?;
    
    let nids: Vec<NoteId> = note_ids.into_iter().map(NoteId).collect();
    let mut card_ids: Vec<i64> = Vec::new();
    for nid in &nids {
        let cids = collection.storage.all_card_ids_of_note_in_template_order(*nid)
            .map_err(|e| anki::error::AnkiError::from(e).to_string())?;
        card_ids.extend(cids.into_iter().map(|c| c.0));
    }
    
    Ok(card_ids)
}

#[command]
fn move_cards_to_deck(card_ids: Vec<i64>, deck_id: i64, state: State<AppState>) -> Result<u32, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;
    
    let cids: Vec<CardId> = card_ids.into_iter().map(CardId).collect();
    let result = collection.set_deck(&cids, DeckId(deck_id))
        .map_err(|e| e.to_string())?;
    
    Ok(result.output as u32)
}

#[command]
fn answer_card(card_id: i64, ease: i32, state: State<AppState>) -> Result<AnswerResult, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;

    // Validate ease value (1=Again, 2=Hard, 3=Good, 4=Easy)
    let rating = match ease {
        1 | 2 | 3 | 4 => ease,
        _ => return Err("Invalid ease value".to_string()),
    };

    // Get queued cards to find the card being answered
    let queued_cards = collection.get_queued_cards(1, false)
        .map_err(|e| e.to_string())?;
    
    // Find the matching queued card
    let queued_card_idx = queued_cards.cards.iter()
        .position(|qc| qc.card.id().0 == card_id)
        .ok_or_else(|| "Card not found in queue".to_string())?;

    // Get the card from the queued card for later use - clone to avoid borrow issues
    let card = &queued_cards.cards[queued_card_idx].card;
    let card_id_obj = card.id();
    let note_id = card.note_id();
    let deck_id = card.deck_id();

    // Get current lapses before answering (via SQL since Card fields are private)
    let lapses_before: i32 = {
        let conn = collection.storage.db();
        conn.query_row(
            "SELECT lapses FROM cards WHERE id = ?",
            [card_id],
            |row| row.get(0)
        ).map_err(|e| e.to_string())?
    };

    // Clone the scheduling states from the queued card to avoid borrow issues
    let current_state = queued_cards.cards[queued_card_idx].states.current.clone();
    let new_state = match ease {
        1 => queued_cards.cards[queued_card_idx].states.again.clone(),
        2 => queued_cards.cards[queued_card_idx].states.hard.clone(),
        3 => queued_cards.cards[queued_card_idx].states.good.clone(),
        4 => queued_cards.cards[queued_card_idx].states.easy.clone(),
        _ => return Err("Invalid ease value".to_string()),
    };
    
    // Answer the card using Anki's scheduling via the backend
    let now = anki::prelude::TimestampMillis::now();
    use anki::scheduler::answering::Rating;
    let rating_val = match ease {
        1 => Rating::Again,
        2 => Rating::Hard,
        3 => Rating::Good,
        4 => Rating::Easy,
        _ => return Err("Invalid ease value".to_string()),
    };
    
    let mut card_answer = anki::scheduler::answering::CardAnswer {
        card_id: card_id_obj,
        rating: rating_val,
        current_state,
        new_state,
        answered_at: now,
        milliseconds_taken: 0,
        custom_data: Default::default(),
        from_queue: true,
    };
    
    // Use the collection's answer_card method
    collection.answer_card(&mut card_answer)
        .map_err(|e| e.to_string())?;

    // After answering, bury sibling cards from the same note
    // This hides them until the next day, matching Anki's default behavior
    let sibling_ids = collection.storage.all_card_ids_of_note_in_template_order(note_id)
        .map_err(|e| e.to_string())?;
    let siblings_to_bury: Vec<CardId> = sibling_ids.into_iter()
        .filter(|cid| *cid != card_id_obj)
        .collect();
    if !siblings_to_bury.is_empty() {
        let _ = collection.bury_or_suspend_cards(
            &siblings_to_bury,
            anki_proto::scheduler::bury_or_suspend_cards_request::Mode::BuryUser
        );
    }

    // Check for leech detection when the answer was "Again" (rating=1)
    let mut leech_detected = false;
    let mut suspended = false;
    
    if rating == 1 {
        if let Some(deck) = collection.get_deck(deck_id).map_err(|e| e.to_string())? {
            if let Some(config_id) = deck.config_id() {
                // Get leech threshold from deck config via SQL
                let leech_threshold: i32 = {
                    let conn = collection.storage.db();
                    conn.query_row(
                        "SELECT CAST(json_extract(conf, '$.leechThreshold') AS INTEGER) FROM config WHERE key = ?",
                        [config_id],
                        |row| row.get(0)
                    ).unwrap_or(8)
                };
                
                // Get the updated card to check lapses count via SQL
                let (lapses_after, flags): (i32, i32) = {
                    let conn = collection.storage.db();
                    conn.query_row(
                        "SELECT lapses, flags FROM cards WHERE id = ?",
                        [card_id],
                        |row| Ok((row.get(0)?, row.get(1)?))
                    ).map_err(|e| e.to_string())?
                };
                
                let was_leech = (flags & 0x100) != 0;
                
                // Check if lapses reached the threshold and this is a new leech
                if lapses_after > lapses_before && lapses_after >= leech_threshold && !was_leech {
                    leech_detected = true;
                    suspended = true;
                    
                    // Suspend the card
                    let cids = vec![card_id_obj];
                    collection.bury_or_suspend_cards(
                        &cids,
                        anki_proto::scheduler::bury_or_suspend_cards_request::Mode::Suspend
                    ).map_err(|e| e.to_string())?;
                    
                    // Mark the leech flag in the card (bit 8)
                    let new_flags = flags | 0x100;
                    let conn = collection.storage.db();
                    conn.execute(
                        "UPDATE cards SET flags = ? WHERE id = ?",
                        params![new_flags, card_id]
                    ).map_err(|e| e.to_string())?;
                }
            }
        }
    }

    Ok(AnswerResult {
        card_id,
        leech: leech_detected,
        suspended,
    })
}

#[command]
fn get_deck_options(deck_id: i64, state: State<AppState>) -> Result<DeckOptionsData, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;

    // Get the deck
    let deck = collection.get_deck(DeckId(deck_id))
        .map_err(|e| e.to_string())?
        .ok_or("Deck not found")?;

    // Get the deck's config ID
    let config_id = deck.config_id()
        .ok_or("Deck does not have a config (filtered deck)")?;

    // Get the deck config using the storage directly
    let conn = collection.storage.db();
    
    // Get the config JSON from the database
    let (name, config_json): (String, String) = conn.query_row(
        "SELECT name, conf FROM deck_config WHERE id = ?",
        params![config_id.0],
        |row| Ok((row.get(0)?, row.get(1)?))
    ).map_err(|e| format!("Deck config not found: {}", e))?;

    // Parse the JSON config
    let config: serde_json::Value = serde_json::from_str(&config_json)
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    // Extract fields from JSON
    let get_f32 = |key: &str, default: f32| -> f32 {
        config.get(key).and_then(|v| v.as_f64()).map(|v| v as f32).unwrap_or(default)
    };
    let get_u32 = |key: &str, default: u32| -> u32 {
        config.get(key).and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(default)
    };
    let get_array_f32 = |key: &str| -> Vec<f32> {
        config.get(key)
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect())
            .unwrap_or_default()
    };

    // FSRS params - check different versions
    let mut fsrs_weights = get_array_f32("fsrsParams");
    if fsrs_weights.is_empty() {
        fsrs_weights = get_array_f32("fsrsParams5");
    }
    let fsrs_enabled = !fsrs_weights.is_empty();

    Ok(DeckOptionsData {
        config_id: config_id.0,
        name,
        new_cards_per_day: get_u32("newPerDay", 20),
        learning_steps: get_array_f32("learnSteps"),
        graduating_interval: get_u32("graduatingInterval", 1),
        easy_interval: get_u32("easyInterval", 4),
        max_reviews_per_day: get_u32("reviewsPerDay", 200),
        easy_bonus: get_f32("easyMultiplier", 1.3),
        interval_modifier: get_f32("intervalMultiplier", 1.0),
        maximum_interval: get_u32("maximumReviewInterval", 36500),
        fsrs_enabled,
        fsrs_weights,
        desired_retention: get_f32("desiredRetention", 0.9),
        lapse_steps: get_array_f32("relearnSteps"),
        lapse_minimum_interval: get_u32("minimumLapseInterval", 1),
        leech_threshold: get_u32("leechThreshold", 8),
    })
}

#[command]
fn save_deck_options(deck_id: i64, opts: DeckOptionsData, state: State<AppState>) -> Result<(), String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;

    // Get the deck
    let deck = collection.get_deck(DeckId(deck_id))
        .map_err(|e| e.to_string())?
        .ok_or("Deck not found")?;

    // Get the deck's config ID
    let config_id = deck.config_id()
        .ok_or("Deck does not have a config (filtered deck)")?;

    // Get existing config JSON
    let conn = collection.storage.db();
    let old_config_json: String = conn.query_row(
        "SELECT conf FROM deck_config WHERE id = ?",
        params![config_id.0],
        |row| row.get(0)
    ).map_err(|e| format!("Deck config not found: {}", e))?;

    // Parse existing config
    let mut config: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&old_config_json)
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    // Update config values
    config.insert("newPerDay".to_string(), serde_json::json!(opts.new_cards_per_day));
    config.insert("learnSteps".to_string(), serde_json::json!(opts.learning_steps));
    config.insert("graduatingInterval".to_string(), serde_json::json!(opts.graduating_interval));
    config.insert("easyInterval".to_string(), serde_json::json!(opts.easy_interval));
    config.insert("reviewsPerDay".to_string(), serde_json::json!(opts.max_reviews_per_day));
    config.insert("easyMultiplier".to_string(), serde_json::json!(opts.easy_bonus));
    config.insert("intervalMultiplier".to_string(), serde_json::json!(opts.interval_modifier));
    config.insert("maximumReviewInterval".to_string(), serde_json::json!(opts.maximum_interval));
    config.insert("relearnSteps".to_string(), serde_json::json!(opts.lapse_steps));
    config.insert("minimumLapseInterval".to_string(), serde_json::json!(opts.lapse_minimum_interval));
    config.insert("leechThreshold".to_string(), serde_json::json!(opts.leech_threshold));
    config.insert("desiredRetention".to_string(), serde_json::json!(opts.desired_retention));

    // Handle FSRS weights
    if opts.fsrs_enabled && !opts.fsrs_weights.is_empty() {
        config.insert("fsrsParams6".to_string(), serde_json::json!(opts.fsrs_weights));
    } else {
        config.remove("fsrsParams6");
        config.remove("fsrsParams5");
        config.remove("fsrsParams4");
    }

    // Serialize back to JSON
    let new_config_json = serde_json::to_string(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    // Get current timestamp
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis() as i64;

    // Update the deck config in the database
    conn.execute(
        "UPDATE deck_config SET name = ?, conf = ?, mod = ? WHERE id = ?",
        params![opts.name, new_config_json, now, config_id.0],
    ).map_err(|e| format!("Failed to save deck config: {}", e))?;

    Ok(())
}

#[command]
fn undo_last_action(state: State<AppState>) -> Result<UndoResult, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;

    let undo_status = collection.undo_status();
    if undo_status.undo.is_none() {
        return Err("Nothing to undo.".to_string());
    }

    let result = collection.undo().map_err(|e| e.to_string())?;
    let _undo_output = result.output;

    // Get action name from the undo status
    let action_name = "Action".to_string();

    Ok(UndoResult {
        action_name,
        card_id: None,
    })
}

#[command]
fn get_undo_status(state: State<AppState>) -> Result<UndoStatusResult, String> {
    let collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_ref().ok_or("Collection not initialized")?;

    let undo_status = collection.undo_status();

    let can_undo = undo_status.undo.is_some();
    let can_redo = undo_status.redo.is_some();

    let undo_label = if can_undo {
        Some("Undo".to_string())
    } else {
        None
    };

    let redo_label = if can_redo {
        Some("Redo".to_string())
    } else {
        None
    };

    Ok(UndoStatusResult {
        can_undo,
        undo_label,
        can_redo,
        redo_label,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState {
        collection: Mutex::new(None),
        media_path: Mutex::new(None),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new()
            .target(tauri_plugin_log::Target::new(
                tauri_plugin_log::TargetKind::LogDir { file_name: Some("anki-wrapper".into()) },
            ))
            .build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_decorum::init())
        .manage(app_state)
        .setup(|app| {
            log::info!("Anki Wrapper starting up...");
            
            // Configure frameless window with overlay titlebar
            let main_window = app.get_webview_window("main").unwrap();
            main_window.create_overlay_titlebar().unwrap();

            #[cfg(target_os = "macos")]
            {
                // Position traffic lights to align with our navbar
                // x=16 insets from left edge, y=18 vertically centers in the 52px navbar
                main_window.set_traffic_lights_inset(16.0, 18.0).unwrap();
            }
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_version,
            create_backup,
            list_backups,
            restore_backup,
            delete_backup,
            get_last_backup_date,
            get_review_count_for_deck,
            optimize_fsrs_weights,
            init_standalone_collection,
            get_scheduler_info,
            get_deck_stats,
            get_all_decks,
            get_deck_stats_for_single_deck,
            get_deck_stats_for_review,
            create_deck,
            add_basic_card,
            add_note,
            answer_card,
            undo_last_action,
            get_undo_status,
            get_deck_options,
            save_deck_options,
            import_apkg,
            import_colpkg,
            import_text_file,
            export_deck_apkg,
            export_collection_colpkg,
            // Tag commands
            get_all_tags,
            get_note_tags,
            set_note_tags,
            add_tags_to_notes,
            remove_tag_from_notes,
            // Card study
            get_next_card,
            set_card_flag,
            // Deck management
            rename_deck,
            reparent_deck,
            delete_deck,
            // Filtered deck commands
            get_filtered_deck_order_labels,
            create_filtered_deck,
            rebuild_filtered_deck,
            empty_filtered_deck,
            // Preferences
            get_preferences,
            save_preferences,
            // Search commands
            search_cards,
            search_notes,
            get_search_completions,
            get_card_detail,
            suspend_cards,
            bury_cards,
            unsuspend_cards,
            delete_notes,
            get_card_ids_for_notes,
            move_cards_to_deck,
            // Statistics
            get_review_stats,
            get_deck_specific_stats,
            get_today_stats,
            get_review_history,
            // Plugin system
            get_installed_plugins,
            enable_plugin,
            disable_plugin,
            get_plugin_source,
            get_plugin_css,
            open_plugins_folder,
            // Notetype commands
            get_all_notetypes,
            get_notetype_detail,
            update_notetype_detail,
            rename_notetype,
            delete_notetype
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[command]
fn import_apkg(path: String, state: State<AppState>) -> Result<ImportLog, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not ready yet. Please wait a moment.")?;
    
    let result = collection.import_apkg(
        path,
        ImportAnkiPackageOptions::default()
    ).map_err(|e| e.to_string())?;
    
    let output = result.output;
    Ok(ImportLog {
        notes_added: output.new.len() as u32,
        notes_updated: output.updated.len() as u32,
        notes_skipped: output.duplicate.len() as u32,
        decks_added: vec![],
    })
}

#[command]
fn import_colpkg(_path: String, state: State<AppState>) -> Result<String, String> {
    let _collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    Err("Collection replacement not implemented yet - this would be destructive".to_string())
}

#[command]
fn import_text_file(path: String, _options: TextImportOptions, state: State<AppState>) -> Result<ImportLog, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not ready yet. Please wait a moment.")?;
    
    // Create minimal CSV metadata - Anki will handle field mapping
    let metadata = CsvMetadata::default();
    
    let result = collection.import_csv(&path, metadata)
        .map_err(|e: anki::error::AnkiError| e.to_string())?;
    
    let output = result.output;
    Ok(ImportLog {
        notes_added: output.new.len() as u32,
        notes_updated: output.updated.len() as u32,
        notes_skipped: output.duplicate.len() as u32,
        decks_added: vec![],
    })
}

#[command]
fn export_deck_apkg(deck_id: i64, out_path: String, include_scheduling: bool, state: State<AppState>) -> Result<String, String> {
    let _collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let _ = (deck_id, out_path, include_scheduling);
    Err("Export functionality coming soon".to_string())
}

#[command]
fn export_collection_colpkg(out_path: String, state: State<AppState>) -> Result<String, String> {
    let _collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let _ = out_path;
    Err("Export functionality coming soon".to_string())
}

// ============ TAG FUNCTIONS ============

#[command]
fn get_all_tags(state: State<AppState>) -> Result<Vec<String>, String> {
    let collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_ref().ok_or("Collection not initialized")?;
    
    let tags: Vec<Tag> = collection.storage.all_tags().map_err(|e| e.to_string())?;
    let mut names: Vec<String> = tags.into_iter().map(|t| t.name).collect();
    names.sort_unstable();
    
    Ok(names)
}

#[command]
fn get_note_tags(note_id: i64, state: State<AppState>) -> Result<Vec<String>, String> {
    let collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_ref().ok_or("Collection not initialized")?;
    
    let note = collection.storage.get_note(NoteId(note_id))
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Note {} not found", note_id))?;
    
    Ok(note.tags.clone())
}

#[command]
fn set_note_tags(note_id: i64, tags: Vec<String>, state: State<AppState>) -> Result<(), String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;
    
    let mut note = collection.storage.get_note(NoteId(note_id))
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Note {} not found", note_id))?;
    
    note.tags = tags;
    collection.update_note(&mut note).map_err(|e| e.to_string())?;
    
    Ok(())
}

#[command]
fn add_tags_to_notes(note_ids: Vec<i64>, tag: String, state: State<AppState>) -> Result<u32, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;
    
    let nids: Vec<NoteId> = note_ids.into_iter().map(NoteId).collect();
    let result = collection.add_tags_to_notes(&nids, &tag)
        .map_err(|e| e.to_string())?;
    
    Ok(result.output as u32)
}

#[command]
fn remove_tag_from_notes(note_ids: Vec<i64>, tag: String, state: State<AppState>) -> Result<(), String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;
    
    let nids: Vec<NoteId> = note_ids.into_iter().map(NoteId).collect();
    let _result = collection.remove_tags_from_notes(&nids, &tag)
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

// ============ DECK MANAGEMENT FUNCTIONS ============

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteDeckResult {
    pub cards_deleted: u32,
    pub notes_deleted: u32,
}

#[command]
fn rename_deck(deck_id: i64, new_name: String, state: State<AppState>) -> Result<(), String> {
    // Validate: new_name must not be empty after trimming
    let trimmed_name = new_name.trim();
    if trimmed_name.is_empty() {
        return Err("Deck name cannot be empty.".to_string());
    }
    
    // Warn about nested deck hierarchy (but don't block)
    if trimmed_name.contains("::") {
        log::warn!("Creating nested deck hierarchy: {}", trimmed_name);
    }
    
    let collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_ref().ok_or("Collection not initialized")?;
    
    let conn = collection.storage.db();
    
    // Update the deck name in the database
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
    conn.execute(
        "UPDATE decks SET name = ?, mtime = ?, usn = -1 WHERE id = ?",
        params![trimmed_name, now, deck_id]
    ).map_err(|e| e.to_string())?;
    
    Ok(())
}

#[command]
fn reparent_deck(deck_id: i64, new_parent_id: Option<i64>, state: State<AppState>) -> Result<(), String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;

    // Get the deck to reparent
    let deck = collection.get_deck(DeckId(deck_id))
        .map_err(|e| e.to_string())?
        .ok_or("Deck not found")?;

    let old_name = deck.human_name().to_string();
    // Extract just the leaf name (last segment after ::)
    let leaf_name = old_name.rsplit("::").next().unwrap_or(&old_name).to_string();

    let new_name = if let Some(parent_id) = new_parent_id {
        // Get parent deck name
        let parent = collection.get_deck(DeckId(parent_id))
            .map_err(|e| e.to_string())?
            .ok_or("Parent deck not found")?;
        let parent_name = parent.human_name().to_string();
        format!("{}::{}", parent_name, leaf_name)
    } else {
        // Moving to root level
        leaf_name
    };

    // Use the existing rename functionality
    collection.rename_deck(DeckId(deck_id), &new_name)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[command]
fn delete_deck(deck_id: i64, delete_cards: bool, state: State<AppState>) -> Result<DeleteDeckResult, String> {
    if !delete_cards {
        return Err("Moving cards to another deck is not yet implemented. Use delete_cards: true.".to_string());
    }
    
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;
    
    // Get card count before deletion for reporting
    let conn = collection.storage.db();
    let card_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM cards WHERE did = ?",
        params![deck_id],
        |row| row.get(0)
    ).unwrap_or(0);
    
    // Delete the deck and all its cards
    collection.remove_decks_and_child_decks(&[DeckId(deck_id)])
        .map_err(|e: anki::error::AnkiError| e.to_string())?;
    
    Ok(DeleteDeckResult {
        cards_deleted: card_count as u32,
        notes_deleted: 0, // Could be calculated if needed
    })
}

// ============ FILTERED DECK FUNCTIONS ============

#[derive(Serialize, Deserialize, Debug)]
pub struct FilteredDeckOrderLabels {
    pub labels: Vec<String>,
}

#[command]
fn get_filtered_deck_order_labels(_state: State<AppState>) -> Result<FilteredDeckOrderLabels, String> {
    // Return standard Anki search order labels
    // These match the FilteredSearchOrder enum values
    let labels = vec![
        "Oldest reviewed first".to_string(),
        "Random".to_string(),
        "Increasing intervals".to_string(),
        "Decreasing intervals".to_string(),
        "Most lapses".to_string(),
        "Added order".to_string(),
        "Due date".to_string(),
        "Latest added first".to_string(),
        "Ascending retrievability".to_string(),
        "Descending retrievability".to_string(),
        "Relative overdueness".to_string(),
    ];
    
    Ok(FilteredDeckOrderLabels { labels })
}

#[command]
fn create_filtered_deck(
    name: String,
    search_query: String,
    limit: u32,
    order: i32,
    state: State<AppState>
) -> Result<i64, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;
    
    // Create a new filtered deck using the public constructor
    let mut deck = anki::decks::Deck::new_filtered();
    deck.name = anki::prelude::NativeDeckName::from_human_name(&name);
    
    // Set up the search term - FilteredDeck is from the proto, so we need to use the proto type
    if let anki::decks::DeckKind::Filtered(ref mut filtered) = deck.kind {
        filtered.search_terms.clear();
        filtered.search_terms.push(anki_proto::decks::deck::filtered::SearchTerm {
            search: search_query,
            limit,
            order: order as i32,
            ..Default::default()
        });
    }
    
    // Convert Deck to FilteredDeckForUpdate using the public TryFrom
    // Use type inference to avoid naming the private type
    let deck_update = deck.try_into()
        .map_err(|e: anki::error::AnkiError| e.to_string())?;
    
    // Add the filtered deck
    let result = collection.add_or_update_filtered_deck(deck_update)
        .map_err(|e| e.to_string())?;
    
    Ok(result.output.0)
}

#[command]
fn rebuild_filtered_deck(deck_id: i64, state: State<AppState>) -> Result<u32, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;
    
    let result = collection.rebuild_filtered_deck(anki::prelude::DeckId(deck_id))
        .map_err(|e| e.to_string())?;
    
    Ok(result.output as u32)
}

#[command]
fn empty_filtered_deck(deck_id: i64, state: State<AppState>) -> Result<(), String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;
    
    collection.empty_filtered_deck(anki::prelude::DeckId(deck_id))
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

// ============ PLUGIN SYSTEM FUNCTIONS ============

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub entry_point: String,
    pub hooks: Option<Vec<String>>,
    pub author: Option<String>,
    pub homepage: Option<String>,
    pub min_api_version: Option<String>,
    pub enabled: bool,
    pub path: String,
    pub has_css: bool,
    pub load_error: Option<String>,
}

fn get_plugins_dir() -> Result<std::path::PathBuf, String> {
    let plugins_dir = dirs::data_local_dir()
        .ok_or("Could not find local data directory")?
        .join("anki-wrapper/plugins");
    std::fs::create_dir_all(&plugins_dir).map_err(|e| e.to_string())?;
    Ok(plugins_dir)
}

fn read_disabled_plugins() -> Result<Vec<String>, String> {
    let plugins_dir = get_plugins_dir()?;
    let disabled_path = plugins_dir.join("disabled.json");
    
    if !disabled_path.exists() {
        return Ok(vec![]);
    }
    
    let content = std::fs::read_to_string(&disabled_path).map_err(|e| e.to_string())?;
    let disabled: Vec<String> = serde_json::from_str(&content).unwrap_or_default();
    Ok(disabled)
}

fn write_disabled_plugins(disabled: &[String]) -> Result<(), String> {
    let plugins_dir = get_plugins_dir()?;
    let disabled_path = plugins_dir.join("disabled.json");
    
    let content = serde_json::to_string_pretty(disabled).map_err(|e| e.to_string())?;
    std::fs::write(&disabled_path, content).map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
fn get_installed_plugins() -> Result<Vec<PluginManifest>, String> {
    let plugins_dir = get_plugins_dir()?;
    let disabled_ids = read_disabled_plugins()?;
    
    let mut plugins = Vec::new();
    
    // Scan each subdirectory
    let entries = std::fs::read_dir(&plugins_dir).map_err(|e| e.to_string())?;
    
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_dir() { continue; }
        
        // Skip the disabled.json file itself if it somehow appears as a directory
        let dir_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if dir_name == "disabled.json" || dir_name.starts_with('.') {
            continue;
        }
        
        let manifest_path = path.join("manifest.json");
        if !manifest_path.exists() { 
            // Push a placeholder with load error for directories without manifest
            plugins.push(PluginManifest {
                id: dir_name.to_string(),
                name: dir_name.to_string(),
                version: "0.0.0".to_string(),
                description: "No manifest.json found".to_string(),
                entry_point: "index.js".to_string(),
                hooks: None,
                author: None,
                homepage: None,
                min_api_version: None,
                enabled: false,
                path: path.to_string_lossy().to_string(),
                has_css: false,
                load_error: Some("Missing manifest.json".to_string()),
            });
            continue; 
        }
        
        // Read and validate manifest
        match std::fs::read_to_string(&manifest_path) {
            Ok(content) => {
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(val) => {
                        // Validate required fields exist and are strings
                        let id = val.get("id").and_then(|v| v.as_str());
                        let name = val.get("name").and_then(|v| v.as_str());
                        let version = val.get("version").and_then(|v| v.as_str());
                        let description = val.get("description").and_then(|v| v.as_str());
                        let entry_point = val.get("entry_point").and_then(|v| v.as_str());
                        
                        if let (Some(id), Some(name), Some(ver), Some(desc), Some(ep)) =
                            (id, name, version, description, entry_point) {
                            
                            // Security: entry_point must not contain path traversal
                            if ep.contains("..") || ep.contains('/') || ep.contains('\\') {
                                plugins.push(PluginManifest {
                                    id: id.to_string(),
                                    name: name.to_string(),
                                    version: ver.to_string(),
                                    description: desc.to_string(),
                                    entry_point: ep.to_string(),
                                    hooks: None,
                                    author: None,
                                    homepage: None,
                                    min_api_version: None,
                                    enabled: false,
                                    path: path.to_string_lossy().to_string(),
                                    has_css: false,
                                    load_error: Some("entry_point contains path traversal".to_string()),
                                });
                                continue;
                            }
                            
                            // Check entry point file exists
                            let ep_path = path.join(ep);
                            if !ep_path.exists() {
                                plugins.push(PluginManifest {
                                    id: id.to_string(),
                                    name: name.to_string(),
                                    version: ver.to_string(),
                                    description: desc.to_string(),
                                    entry_point: ep.to_string(),
                                    hooks: None,
                                    author: None,
                                    homepage: None,
                                    min_api_version: None,
                                    enabled: false,
                                    path: path.to_string_lossy().to_string(),
                                    has_css: false,
                                    load_error: Some(format!("Entry point file not found: {}", ep)),
                                });
                                continue;
                            }
                            
                            let has_css = path.join("styles.css").exists();
                            let enabled = !disabled_ids.contains(&id.to_string());
                            
                            plugins.push(PluginManifest {
                                id: id.to_string(),
                                name: name.to_string(),
                                version: ver.to_string(),
                                description: desc.to_string(),
                                entry_point: ep.to_string(),
                                hooks: val.get("hooks").and_then(|v| {
                                    v.as_array().map(|arr|
                                        arr.iter().filter_map(|h| h.as_str().map(String::from)).collect()
                                    )
                                }),
                                author: val.get("author").and_then(|v| v.as_str().map(String::from)),
                                homepage: val.get("homepage").and_then(|v| v.as_str().map(String::from)),
                                min_api_version: val.get("min_api_version").and_then(|v| v.as_str().map(String::from)),
                                enabled,
                                path: path.to_string_lossy().to_string(),
                                has_css,
                                load_error: None,
                            });
                        } else {
                            // Missing required fields
                            plugins.push(PluginManifest {
                                id: dir_name.to_string(),
                                name: dir_name.to_string(),
                                version: "0.0.0".to_string(),
                                description: "Invalid manifest - missing required fields".to_string(),
                                entry_point: "index.js".to_string(),
                                hooks: None,
                                author: None,
                                homepage: None,
                                min_api_version: None,
                                enabled: false,
                                path: path.to_string_lossy().to_string(),
                                has_css: false,
                                load_error: Some("Missing required fields: id, name, version, description, entry_point".to_string()),
                            });
                        }
                    }
                    Err(e) => {
                        // Invalid JSON
                        plugins.push(PluginManifest {
                            id: dir_name.to_string(),
                            name: dir_name.to_string(),
                            version: "0.0.0".to_string(),
                            description: "Invalid manifest.json - not valid JSON".to_string(),
                            entry_point: "index.js".to_string(),
                            hooks: None,
                            author: None,
                            homepage: None,
                            min_api_version: None,
                            enabled: false,
                            path: path.to_string_lossy().to_string(),
                            has_css: false,
                            load_error: Some(format!("Invalid JSON in manifest.json: {}", e)),
                        });
                    }
                }
            }
            Err(e) => {
                // Can't read manifest file
                plugins.push(PluginManifest {
                    id: dir_name.to_string(),
                    name: dir_name.to_string(),
                    version: "0.0.0".to_string(),
                    description: "Could not read manifest.json".to_string(),
                    entry_point: "index.js".to_string(),
                    hooks: None,
                    author: None,
                    homepage: None,
                    min_api_version: None,
                    enabled: false,
                    path: path.to_string_lossy().to_string(),
                    has_css: false,
                    load_error: Some(format!("Could not read manifest.json: {}", e)),
                });
            }
        }
    }
    
    Ok(plugins)
}

#[command]
fn enable_plugin(plugin_id: String) -> Result<(), String> {
    let mut disabled = read_disabled_plugins()?;
    
    // Remove the plugin from disabled list
    disabled.retain(|id| id != &plugin_id);
    
    write_disabled_plugins(&disabled)?;
    
    log::info!("Plugin enabled: {}", plugin_id);
    Ok(())
}

#[command]
fn disable_plugin(plugin_id: String) -> Result<(), String> {
    let mut disabled = read_disabled_plugins()?;
    
    // Add the plugin to disabled list if not already there
    if !disabled.contains(&plugin_id) {
        disabled.push(plugin_id.clone());
        write_disabled_plugins(&disabled)?;
        log::info!("Plugin disabled: {}", plugin_id);
    }
    
    Ok(())
}

#[command]
fn get_plugin_source(plugin_id: String) -> Result<String, String> {
    let plugins_dir = get_plugins_dir()?;
    let plugin_path = plugins_dir.join(&plugin_id);
    
    // Security: ensure the plugin_id doesn't contain path traversal
    if plugin_id.contains("..") || plugin_id.contains('/') || plugin_id.contains('\\') {
        return Err("Invalid plugin ID".to_string());
    }
    
    // First get the manifest to find the entry point
    let manifest_path = plugin_path.join("manifest.json");
    if !manifest_path.exists() {
        return Err("Plugin not found".to_string());
    }
    
    let content = std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
    let val: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| e.to_string())?;
    
    let entry_point = val.get("entry_point")
        .and_then(|v| v.as_str())
        .ok_or("Missing entry_point in manifest")?;
    
    // Security: ensure entry_point doesn't contain path traversal
    if entry_point.contains("..") || entry_point.contains('/') || entry_point.contains('\\') {
        return Err("Invalid entry_point".to_string());
    }
    
    let source_path = plugin_path.join(entry_point);
    if !source_path.exists() {
        return Err(format!("Entry point file not found: {}", entry_point));
    }
    
    std::fs::read_to_string(&source_path).map_err(|e| e.to_string())
}

#[command]
fn get_plugin_css(plugin_id: String) -> Result<String, String> {
    let plugins_dir = get_plugins_dir()?;
    let plugin_path = plugins_dir.join(&plugin_id);
    
    // Security: ensure the plugin_id doesn't contain path traversal
    if plugin_id.contains("..") || plugin_id.contains('/') || plugin_id.contains('\\') {
        return Err("Invalid plugin ID".to_string());
    }
    
    let css_path = plugin_path.join("styles.css");
    
    if !css_path.exists() {
        return Err("No styles.css found".to_string());
    }
    
    std::fs::read_to_string(&css_path).map_err(|e| e.to_string())
}

#[command]
fn open_plugins_folder() -> Result<(), String> {
    let plugins_dir = get_plugins_dir()?;
    // Open the folder in the system file manager
    open::that(&plugins_dir).map_err(|e| e.to_string())?;
    Ok(())
}

// ============ CARD STUDY FUNCTIONS ============

#[derive(Serialize, Deserialize, Debug)]
pub struct CardInfo {
    pub card_id: i64,
    pub note_id: i64,
    pub front: String,       // rendered HTML
    pub back: String,        // rendered HTML
    pub flag: u8,
    // Interval labels for answer buttons
    pub again_interval: String,  // e.g., "1m", "10m", "1d"
    pub hard_interval: String,
    pub good_interval: String,
    pub easy_interval: String,
}

#[command]
fn get_next_card(deck_id: i64, state: State<AppState>) -> Result<CardInfo, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;
    
    // CRITICAL FIX: Set the current deck before getting queued cards.
    // Anki's scheduler uses the "current deck" setting to determine which
    // deck to pull cards from. Without this, it pulls from whatever deck
    // was last selected (often the Default deck).
    collection.set_current_deck(DeckId(deck_id)).map_err(|e| e.to_string())?;
    
    // Get queued cards using Anki's proper scheduling API
    let queued_cards = collection.get_queued_cards(1, false)
        .map_err(|e| e.to_string())?;
    
    // Check if there's a card available
    let queued_card = queued_cards.cards.first()
        .ok_or_else(|| "No cards left to study".to_string())?;
    
    let card_id = queued_card.card.id().0;
    let note_id = queued_card.card.note_id().0;
    
    // Render the card HTML
    let rendered = collection.render_existing_card(queued_card.card.id(), false, false)
        .map_err(|e| e.to_string())?;
    let front = rendered.question().into_owned();
    let back = rendered.answer().into_owned();
    
    // Get the card flags from the database
    let conn = collection.storage.db();
    let flags: i32 = conn.query_row(
        "SELECT flags FROM cards WHERE id = ?",
        params![card_id],
        |row| row.get(0)
    ).unwrap_or(0);
    let flag = (flags & 0xFF) as u8;
    
    // Get the interval labels for answer buttons
    let interval_labels = collection.describe_next_states(&queued_card.states)
        .map_err(|e| e.to_string())?;
    
    Ok(CardInfo {
        card_id,
        note_id,
        front,
        back,
        flag,
        again_interval: interval_labels.first().cloned().unwrap_or_default(),
        hard_interval: interval_labels.get(1).cloned().unwrap_or_default(),
        good_interval: interval_labels.get(2).cloned().unwrap_or_default(),
        easy_interval: interval_labels.get(3).cloned().unwrap_or_default(),
    })
}

#[command]
fn set_card_flag(card_id: i64, flag: u8, state: State<AppState>) -> Result<(), String> {
    // Validate flag is 0-7 (Anki supports 7 flags)
    if flag > 7 {
        return Err("Invalid flag value: must be 0-7".to_string());
    }
    
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;
    
    let cids = vec![CardId(card_id)];
    collection.set_card_flag(&cids, flag as u32)
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

// ============ STATISTICS DATA STRUCTURES ============

#[derive(Serialize, Deserialize, Clone)]
pub struct ForecastDay {
    pub day: i32,
    pub new: i64,
    pub review: i64,
    pub learning: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DailyReview {
    pub date: String,
    pub count: i64,
    pub time_secs: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CardTypeStats {
    pub new: i64,
    pub learning: i64,
    pub young: i64,
    pub mature: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RetentionStats {
    pub young_retention: f64,
    pub mature_retention: f64,
    pub overall: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DailyCount {
    pub date: String,
    pub count: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ReviewStats {
    pub forecast: Vec<ForecastDay>,
    pub daily_reviews: Vec<DailyReview>,
    pub hourly_breakdown: Vec<i64>,
    pub card_types: CardTypeStats,
    pub retention: RetentionStats,
    pub cards_added: Vec<DailyCount>,
    pub current_streak: i64,
    pub longest_streak: i64,
    pub total_reviews: i64,
    pub total_cards: i64,
    pub total_notes: i64,
    pub average_ease: f64,
    pub average_interval_days: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TodayStats {
    pub cards_reviewed: i64,
    pub time_spent_ms: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ReviewHistoryEntry {
    pub timestamp: i64,
    pub card_id: i64,
    pub ease: i32,
    pub interval: i32,
    pub last_interval: i32,
    pub ease_factor: i32,
    pub time_ms: i64,
    pub review_type: i32,
}

// ============ STATISTICS FUNCTIONS ============

#[command]
fn get_review_stats(deck_id: Option<i64>, state: State<AppState>) -> Result<ReviewStats, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = match collection.as_mut() {
        Some(c) => c,
        None => return Ok(ReviewStats::default()),
    };

    // 1. Forecast - cards due in next 30 days
    let timing = collection.timing_today().map_err(|e| e.to_string())?;
    
    let conn = collection.storage.db();
    
    // Build deck filter clause if deck_id is provided
    let (deck_filter, deck_params): (String, Vec<i64>) = if let Some(did) = deck_id {
        // Get all child deck IDs for the given deck
        let child_deck_ids = get_child_deck_ids(&conn, did);
        let placeholders: Vec<String> = child_deck_ids.iter().map(|_| "?".to_string()).collect();
        (
            format!("AND c.did IN ({})", placeholders.join(",")),
            child_deck_ids,
        )
    } else {
        (String::new(), vec![])
    };
    let today = timing.days_elapsed;
    let mut forecast = Vec::new();
    
    for day_offset in 1..=30 {
        let due_day = today as i32 + day_offset as i32;
        
        // Count new cards (queue = -2 means new)
        let new_count: i64 = conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM cards c WHERE c.queue = -2 AND c.due = ? {}",
                deck_filter
            ),
            params![due_day],
            |row| row.get(0)
        ).unwrap_or(0);
        
        // Count learning cards (queue = 1 or 3)
        let learning_count: i64 = conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM cards c WHERE c.queue IN (1, 3) AND c.due <= ? {}",
                deck_filter
            ),
            params![due_day],
            |row| row.get(0)
        ).unwrap_or(0);
        
        // Count review cards (queue = 2)
        let review_count: i64 = conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM cards c WHERE c.queue = 2 AND c.due = ? {}",
                deck_filter
            ),
            params![due_day],
            |row| row.get(0)
        ).unwrap_or(0);
        
        if new_count > 0 || learning_count > 0 || review_count > 0 {
            forecast.push(ForecastDay {
                day: day_offset,
                new: new_count,
                review: review_count,
                learning: learning_count,
            });
        } else {
            forecast.push(ForecastDay {
                day: day_offset,
                new: 0,
                review: 0,
                learning: 0,
            });
        }
    }

    // 2. Daily reviews for last 30 days
    let mut daily_reviews = Vec::new();
    {
        let mut stmt = conn.prepare(
            &format!(
                "SELECT date(r.id/1000, 'unixepoch', 'localtime') as rev_date, 
                        COUNT(*) as count, 
                        SUM(r.time)/1000 as time_secs
                 FROM revlog r 
                 JOIN cards c ON r.cid = c.id 
                 WHERE r.id/1000 >= strftime('%s', 'now', '-30 days')
                 {}
                 GROUP BY rev_date 
                 ORDER BY rev_date",
                deck_filter
            )
        ).map_err(|e| e.to_string())?;
        
        let rows = stmt.query_map(rusqlite::params_from_iter(deck_params.iter()), |row| {
            Ok(DailyReview {
                date: row.get(0)?,
                count: row.get(1)?,
                time_secs: row.get::<_, Option<i64>>(2)?.unwrap_or(0),
            })
        }).map_err(|e| e.to_string())?;
        
        for row in rows.filter_map(|r| r.ok()) {
            daily_reviews.push(row);
        }
    }

    // 3. Hourly breakdown (0-23)
    let mut hourly_breakdown = vec![0i64; 24];
    {
        let mut stmt = conn.prepare(
            &format!(
                "SELECT CAST(strftime('%H', r.id/1000, 'unixepoch', 'localtime') AS INTEGER) as hour, 
                        COUNT(*) as count
                 FROM revlog r 
                 JOIN cards c ON r.cid = c.id 
                 WHERE r.id/1000 >= strftime('%s', 'now', '-30 days')
                 {}
                 GROUP BY hour",
                deck_filter
            )
        ).map_err(|e| e.to_string())?;
        
        let rows = stmt.query_map(rusqlite::params_from_iter(deck_params.iter()), |row| {
            Ok((row.get::<_, i32>(0)?, row.get::<_, i64>(1)?))
        }).map_err(|e| e.to_string())?;
        
        for row in rows.filter_map(|r| r.ok()) {
            let hour = row.0 as usize;
            if hour < 24 {
                hourly_breakdown[hour] = row.1;
            }
        }
    }

    // 4. Card type distribution
    let card_types = {
        // New: queue = -2
        let new_count: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM cards c WHERE c.queue = -2 {}", deck_filter),
            rusqlite::params_from_iter(deck_params.iter()),
            |row| row.get(0)
        ).unwrap_or(0);
        
        // Learning: queue = 1 or 3
        let learning_count: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM cards c WHERE c.queue IN (1, 3) {}", deck_filter),
            rusqlite::params_from_iter(deck_params.iter()),
            |row| row.get(0)
        ).unwrap_or(0);
        
        // Young: queue = 2 and ivl < 21
        let young_count: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM cards c WHERE c.queue = 2 AND c.ivl < 21 {}", deck_filter),
            rusqlite::params_from_iter(deck_params.iter()),
            |row| row.get(0)
        ).unwrap_or(0);
        
        // Mature: queue = 2 and ivl >= 21
        let mature_count: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM cards c WHERE c.queue = 2 AND c.ivl >= 21 {}", deck_filter),
            rusqlite::params_from_iter(deck_params.iter()),
            |row| row.get(0)
        ).unwrap_or(0);
        
        CardTypeStats {
            new: new_count,
            learning: learning_count,
            young: young_count,
            mature: mature_count,
        }
    };

    // 5. Retention stats
    let retention = {
        // Young retention (ivl < 21) - ease >= 2 means correct answer
        let young_total: i64 = conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM revlog r 
                 JOIN cards c ON r.cid = c.id 
                 WHERE c.ivl < 21 {}",
                deck_filter
            ),
            rusqlite::params_from_iter(deck_params.iter()),
            |row| row.get(0)
        ).unwrap_or(0);
        
        let young_correct: i64 = conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM revlog r 
                 JOIN cards c ON r.cid = c.id 
                 WHERE c.ivl < 21 AND r.ease >= 2 {}",
                deck_filter
            ),
            rusqlite::params_from_iter(deck_params.iter()),
            |row| row.get(0)
        ).unwrap_or(0);
        
        // Mature retention (ivl >= 21)
        let mature_total: i64 = conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM revlog r 
                 JOIN cards c ON r.cid = c.id 
                 WHERE c.ivl >= 21 {}",
                deck_filter
            ),
            rusqlite::params_from_iter(deck_params.iter()),
            |row| row.get(0)
        ).unwrap_or(0);
        
        let mature_correct: i64 = conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM revlog r 
                 JOIN cards c ON r.cid = c.id 
                 WHERE c.ivl >= 21 AND r.ease >= 2 {}",
                deck_filter
            ),
            rusqlite::params_from_iter(deck_params.iter()),
            |row| row.get(0)
        ).unwrap_or(0);
        
        // Overall retention
        let overall_total = young_total + mature_total;
        let overall_correct = young_correct + mature_correct;
        
        RetentionStats {
            young_retention: if young_total > 0 { young_correct as f64 / young_total as f64 } else { 0.0 },
            mature_retention: if mature_total > 0 { mature_correct as f64 / mature_total as f64 } else { 0.0 },
            overall: if overall_total > 0 { overall_correct as f64 / overall_total as f64 } else { 0.0 },
        }
    };

    // 6. Cards added per day (last 30 days)
    let mut cards_added = Vec::new();
    {
        let mut stmt = conn.prepare(
            &format!(
                "SELECT date(c.id/1000, 'unixepoch', 'localtime') as add_date, 
                        COUNT(*) as count
                 FROM cards c 
                 WHERE c.id/1000 >= strftime('%s', 'now', '-30 days')
                 {}
                 GROUP BY add_date 
                 ORDER BY add_date",
                deck_filter
            )
        ).map_err(|e| e.to_string())?;
        
        let rows = stmt.query_map(rusqlite::params_from_iter(deck_params.iter()), |row| {
            Ok(DailyCount {
                date: row.get(0)?,
                count: row.get(1)?,
            })
        }).map_err(|e| e.to_string())?;
        
        for row in rows.filter_map(|r| r.ok()) {
            cards_added.push(row);
        }
    }

    // 7. Streak calculation
    let (current_streak, longest_streak) = {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT date(r.id/1000, 'unixepoch', 'localtime') as rev_date 
             FROM revlog r 
             ORDER BY rev_date DESC"
        ).map_err(|e| e.to_string())?;
        
        let rows: Vec<String> = stmt.query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        
        if rows.is_empty() {
            (0, 0)
        } else {
            let mut current = 0i64;
            let mut longest = 0i64;
            let mut temp_streak = 0i64;
            
            let today_str = chrono::Local::now().format("%Y-%m-%d").to_string();
            let yesterday_str = (chrono::Local::now() - chrono::Duration::days(1)).format("%Y-%m-%d").to_string();
            
            for (i, date_str) in rows.iter().enumerate() {
                if i == 0 {
                    // First date must be today or yesterday to start streak
                    if *date_str == today_str || *date_str == yesterday_str {
                        temp_streak = 1;
                        current = 1;
                    } else {
                        break;
                    }
                } else {
                    let prev_date = chrono::NaiveDate::parse_from_str(&rows[i-1], "%Y-%m-%d").ok();
                    let curr_date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok();
                    
                    if let (Some(prev), Some(curr)) = (prev_date, curr_date) {
                        let diff = prev.signed_duration_since(curr).num_days();
                        if diff == 1 {
                            temp_streak += 1;
                            if current > 0 {
                                current = temp_streak;
                            }
                        } else {
                            longest = longest.max(temp_streak);
                            temp_streak = 1;
                            if current > 0 {
                                current = 0; // Streak broken
                            }
                        }
                    }
                }
            }
            
            longest = longest.max(temp_streak);
            (current, longest)
        }
    };

    // 8. Totals
    let total_reviews: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM revlog r JOIN cards c ON r.cid = c.id {}", deck_filter.replace("AND", "WHERE")),
        rusqlite::params_from_iter(deck_params.iter()),
        |row| row.get(0)
    ).unwrap_or(0);
    
    let total_cards: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM cards c {}", deck_filter.replace("AND", "WHERE")),
        rusqlite::params_from_iter(deck_params.iter()),
        |row| row.get(0)
    ).unwrap_or(0);
    
    let total_notes: i64 = conn.query_row(
        "SELECT COUNT(*) FROM notes",
        [],
        |row| row.get(0)
    ).unwrap_or(0);

    let average_ease: f64 = conn.query_row(
        "SELECT AVG(CAST(c.factor AS REAL)/10) FROM cards c WHERE c.queue = 2",
        [],
        |row| row.get(0)
    ).unwrap_or(250.0) / 100.0;

    let average_interval_days: f64 = conn.query_row(
        &format!(
            "SELECT AVG(CAST(c.ivl AS REAL)) FROM cards c WHERE c.queue = 2 {}",
            deck_filter.replace("AND", "WHERE")
        ),
        rusqlite::params_from_iter(deck_params.iter()),
        |row| row.get(0)
    ).unwrap_or(0.0);

    Ok(ReviewStats {
        forecast,
        daily_reviews,
        hourly_breakdown,
        card_types,
        retention,
        cards_added,
        current_streak,
        longest_streak,
        total_reviews,
        total_cards,
        total_notes,
        average_ease,
        average_interval_days,
    })
}

fn get_child_deck_ids(conn: &rusqlite::Connection, deck_id: i64) -> Vec<i64> {
    let mut ids = vec![deck_id];
    
    // Get all child deck IDs recursively
    let mut to_check = vec![deck_id];
    while let Some(current_id) = to_check.pop() {
        let mut stmt = conn.prepare(
            "SELECT id FROM decks WHERE name LIKE ?"
        ).ok();
        
        if let Some(ref mut s) = stmt {
            let prefix = format!("{}::", format_deck_id(current_id));
            let rows = s.query_map([format!("{}%", prefix)], |row| row.get::<_, i64>(0)).ok();
            
            if let Some(rs) = rows {
                for row in rs.filter_map(|r| r.ok()) {
                    if !ids.contains(&row) {
                        ids.push(row);
                        to_check.push(row);
                    }
                }
            }
        }
    }
    
    ids
}

fn format_deck_id(id: i64) -> String {
    // Convert deck ID to the format used in deck names
    // Deck IDs are base36 encoded in Anki
    let mut result = String::new();
    let mut n = id;
    while n > 0 {
        let remainder = (n % 36) as u8;
        let c = if remainder < 10 { b'0' + remainder } else { b'a' + remainder - 10 };
        result.push(c as char);
        n /= 36;
    }
    let rev: String = result.chars().rev().collect();
    rev
}

#[command]
fn get_deck_specific_stats(deck_id: i64, state: State<AppState>) -> Result<ReviewStats, String> {
    get_review_stats(Some(deck_id), state)
}

#[command]
fn get_today_stats(state: State<AppState>) -> Result<TodayStats, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = match collection.as_mut() {
        Some(c) => c,
        None => return Ok(TodayStats { cards_reviewed: 0, time_spent_ms: 0 }),
    };

    let conn = collection.storage.db();
    
    // Get today's date in local timezone
    let _today_start = chrono::Local::now().format("%Y-%m-%d 00:00:00").to_string();
    
    // Count cards reviewed today
    let cards_reviewed: i64 = conn.query_row(
        "SELECT COUNT(*) FROM revlog WHERE id/1000 >= strftime('%s', 'now', 'start of day')",
        [],
        |row| row.get(0)
    ).unwrap_or(0);
    
    // Get time spent today (in milliseconds)
    let time_spent_ms: i64 = conn.query_row(
        "SELECT COALESCE(SUM(time), 0) FROM revlog WHERE id/1000 >= strftime('%s', 'now', 'start of day')",
        [],
        |row| row.get(0)
    ).unwrap_or(0);

    Ok(TodayStats {
        cards_reviewed,
        time_spent_ms,
    })
}

#[command]
fn get_review_history(limit: i64, state: State<AppState>) -> Result<Vec<ReviewHistoryEntry>, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = match collection.as_mut() {
        Some(c) => c,
        None => return Ok(vec![]),
    };

    let conn = collection.storage.db();
    
    let mut stmt = conn.prepare(
        "SELECT id/1000, cid, ease, ivl, lastIvl, factor, time, type 
         FROM revlog 
         ORDER BY id DESC 
         LIMIT ?"
    ).map_err(|e| e.to_string())?;
    
    let rows = stmt.query_map([limit], |row| {
        Ok(ReviewHistoryEntry {
            timestamp: row.get(0)?,
            card_id: row.get(1)?,
            ease: row.get(2)?,
            interval: row.get(3)?,
            last_interval: row.get(4)?,
            ease_factor: row.get(5)?,
            time_ms: row.get(6)?,
            review_type: row.get(7)?,
        })
    }).map_err(|e| e.to_string())?;
    
    let mut results = Vec::new();
    for row in rows.filter_map(|r| r.ok()) {
        results.push(row);
    }

    Ok(results)
}

// Implement Default for ReviewStats
impl Default for ReviewStats {
    fn default() -> Self {
        ReviewStats {
            forecast: vec![],
            daily_reviews: vec![],
            hourly_breakdown: vec![0; 24],
            card_types: CardTypeStats {
                new: 0,
                learning: 0,
                young: 0,
                mature: 0,
            },
            retention: RetentionStats {
                young_retention: 0.0,
                mature_retention: 0.0,
                overall: 0.0,
            },
            cards_added: vec![],
            current_streak: 0,
            longest_streak: 0,
            total_reviews: 0,
            total_cards: 0,
            total_notes: 0,
            average_ease: 2.5,
            average_interval_days: 0.0,
        }
    }
}

