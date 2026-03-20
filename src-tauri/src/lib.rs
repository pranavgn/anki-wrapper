use std::sync::Mutex;

// ============ PREFERENCES ============

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppPreferences {
    pub animations_enabled: bool,
    pub reduce_motion: bool,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self { 
            animations_enabled: true, 
            reduce_motion: false 
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
use tauri::{command, State, AppHandle, Emitter};
use serde::{Deserialize, Serialize};
use rusqlite::params;
use serde_json;
use regex;
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

#[derive(Serialize, Deserialize, Debug)]
pub struct DeckInfo {
    pub id: i64,
    pub name: String,
    pub card_count: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeckStats {
    pub new: usize,
    pub review: usize,
    pub learning: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CardData {
    pub front: String,
    pub back: String,
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

#[command]
async fn init_standalone_collection(app_handle: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    // Use app data directory for standalone collection
    let app_data_dir = dirs::data_local_dir()
        .ok_or("Could not find local data directory")?
        .join("anki-wrapper");
    
    // Create directory if it doesn't exist
    std::fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;
    
    let db_path = app_data_dir.join("collection.anki2");
    
    // Open or create the collection using CollectionBuilder
    let media_dir = app_data_dir.join("media");
    let _ = std::fs::create_dir_all(&media_dir);
    let media_db_path = app_data_dir.join("media.db");
    let _ = std::fs::create_dir_all(&app_data_dir);
    
    let collection = CollectionBuilder::new(&db_path)
        .set_media_paths(media_dir.clone(), media_db_path)
        .build()
        .map_err(|e| {
            log::error!("Failed to open/create collection: {}", e);
            e.to_string()
        })?;
    
    log::info!("Collection opened/created at {:?}", db_path);
    
    // Store in state
    {
        let mut col = state.collection.lock().map_err(|_| "Failed to lock collection")?;
        *col = Some(collection);
        // Store media path
        let mut media_path = state.media_path.lock().map_err(|_| "Failed to lock media path")?;
        *media_path = Some(media_dir);
    }
    
    // Initialize FSRS - wrap in its own scope so we release the lock
    {
        let mut col_guard = state.collection.lock().map_err(|_| "Failed to lock collection")?;
        if let Some(col) = col_guard.as_mut() {
            // Step 1: Enable Sched2021 (v3 scheduler) 
            if let Err(e) = col.set_config_bool(BoolKey::Sched2021, true, false) {
                eprintln!("FSRS init warning: Failed to enable Sched2021: {}", e);
            } else {
                log::info!("Enabled Sched2021 (v3 scheduler)");
            }
            
            // Step 2: Enable FSRS globally
            if let Err(e) = col.set_config_bool(BoolKey::Fsrs, true, false) {
                eprintln!("FSRS init warning: Failed to enable FSRS: {}", e);
            } else {
                log::info!("Enabled FSRS globally");
            }
        }
    }
    
    let _ = app_handle.emit("collection:ready", CollectionReadyPayload { success: true, error: None });
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
fn get_decks(state: State<AppState>) -> Result<Vec<DeckInfo>, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = match collection.as_mut() {
        Some(c) => c,
        None => return Ok(vec![]),
    };

    let tree = collection.deck_tree(None).map_err(|e| e.to_string())?;
    
    let mut deck_infos = Vec::new();
    flatten_deck_tree(tree, &mut deck_infos);
    
    Ok(deck_infos)
}

fn flatten_deck_tree(node: DeckTreeNode, decks: &mut Vec<DeckInfo>) {
    // Skip the root node (level 0) which doesn't represent a real deck
    if node.level > 0 {
        decks.push(DeckInfo {
            id: node.deck_id as i64,
            name: node.name,
            card_count: node.total_in_deck as usize,
        });
    }
    for child in node.children {
        flatten_deck_tree(child, decks);
    }
}

#[command]
fn get_deck_stats(deck_id: i64, state: State<AppState>) -> Result<DeckStats, String> {
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
    get_deck_stats(deck_id, state)
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
    let result = collection.add_note(&mut note, anki::prelude::DeckId(deck_id))
        .map_err(|e| e.to_string())?;

    Ok(note.id.0)
}

// ==================== Media Commands ====================

#[command]
fn save_media_file(filename: String, data: Vec<u8>, state: State<AppState>) -> Result<String, String> {
    let collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_ref().ok_or("Collection not initialized")?;
    
    let mut mgr = collection.media().map_err(|e| e.to_string())?;
    let actual_filename = mgr.add_file(&filename, &data)
        .map_err(|e| e.to_string())?;
    
    Ok(actual_filename.into_owned())
}

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
                SUBSTR(n.flds, 1, INSTR(n.flds, CHAR(0)) - 1) as front_preview,
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

        // Get deck name from first card
        let mut deck_name = "Default".to_string();
        if let Ok(cards) = collection.storage.all_card_ids_of_note_in_template_order(*nid) {
            if let Some(first_card_id) = cards.first() {
                if let Ok(Some(card)) = collection.storage.get_card(*first_card_id) {
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

        // Calculate created_days_ago
        let created_days_ago = (note.id.0 / 86400000) as u32;

        // Get card count
        let card_count = fields.len() as u32; // This is approximate

        results.push(NoteRow {
            note_id: nid.0,
            first_card_id: 0, // Would need another query to get this accurately
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
fn get_cards_for_study(deck_id: i64, limit: usize, state: State<AppState>) -> Result<Vec<CardData>, String> {
    let collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = match collection.as_ref() {
        Some(c) => c,
        None => return Ok(vec![]),
    };

    let conn = collection.storage.db();

    let mut stmt = conn.prepare("SELECT n.flds FROM cards c JOIN notes n ON c.nid = n.id WHERE c.did = ? LIMIT ?").map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![deck_id, limit as i64], |row| {
        let flds: String = row.get(0)?;
        Ok(flds)
    }).map_err(|e| e.to_string())?;

    let mut cards_data = Vec::new();
    for flds_result in rows {
        let flds = flds_result.map_err(|e| e.to_string())?;
        let fields: Vec<String> = serde_json::from_str(&flds).map_err(|e| e.to_string())?;
        if fields.len() >= 2 {
            cards_data.push(CardData {
                front: fields[0].clone(),
                back: fields[1].clone(),
            });
        }
    }

    Ok(cards_data)
}

#[command]
fn answer_card(card_id: i64, ease: i32, state: State<AppState>) -> Result<AnswerResult, String> {
    let mut collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_mut().ok_or("Collection not initialized")?;

    // Validate ease value
    let _rating = match ease {
        1 | 2 | 3 | 4 => {},
        _ => return Err("Invalid ease value".to_string()),
    };

    // For now, return basic result - full answer implementation requires more complex API
    // The actual answering is handled by the frontend through get_next_card
    Ok(AnswerResult {
        card_id,
        leech: false,
        suspended: false,
    })
}

#[command]
fn get_deck_options(deck_id: i64, state: State<AppState>) -> Result<DeckOptionsData, String> {
    let collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let _collection = collection.as_ref().ok_or("Collection not initialized")?;

    // Return default options for now - the API requires more investigation
    Ok(DeckOptionsData {
        config_id: deck_id,
        name: "Default".to_string(),
        new_cards_per_day: 20,
        learning_steps: vec![1.0, 10.0],
        graduating_interval: 1,
        easy_interval: 4,
        max_reviews_per_day: 200,
        easy_bonus: 1.3,
        interval_modifier: 1.0,
        maximum_interval: 36500,
        fsrs_enabled: true,
        fsrs_weights: vec![],
        desired_retention: 0.9,
        lapse_steps: vec![10.0],
        lapse_minimum_interval: 1,
        leech_threshold: 8,
    })
}

#[command]
fn save_deck_options(_deck_id: i64, _opts: DeckOptionsData, state: State<AppState>) -> Result<(), String> {
    let _collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let _collection = _collection.as_ref().ok_or("Collection not initialized")?;

    // Save functionality would go here - requires more API investigation
    // For now, just return success
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
        .manage(app_state)
        .setup(|_app| {
            log::info!("Anki Wrapper starting up...");
            
            // Try to find Anki data directory
            let home_dir = dirs::home_dir().expect("Could not find home directory");
            let anki_dir = home_dir.join(".local/share/Anki2");
            
            if anki_dir.exists() {
                log::info!("Found Anki data directory: {:?}", anki_dir);
                let db_path = anki_dir.join("collection.anki2");
                
                if db_path.exists() {
                    log::info!("Found Anki database: {:?}", db_path);
                    // Note: Opening Anki's database directly requires special handling
                    // The collection should be opened through Anki's API for proper locking
                    // For now, we'll note this in the state
                } else {
                    log::warn!("Anki database not found at {:?}", db_path);
                }
            } else {
                log::warn!("Anki data directory not found at {:?}", anki_dir);
            }
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_version,
            init_standalone_collection,
            get_scheduler_info,
            get_decks,
            get_deck_stats,
            get_deck_stats_for_review,
            create_deck,
            add_basic_card,
            get_cards_for_study,
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
            delete_deck,
            // Preferences
            get_preferences,
            save_preferences,
            // Search commands
            search_cards,
            search_notes,
            get_card_detail,
            suspend_cards,
            unsuspend_cards,
            delete_notes,
            get_card_ids_for_notes,
            move_cards_to_deck
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

// ============ CARD STUDY FUNCTIONS ============

#[derive(Serialize, Deserialize, Debug)]
pub struct CardInfo {
    pub card_id: i64,
    pub note_id: i64,
    pub front: String,
    pub back: String,
    pub flag: u8,
}

#[command]
fn get_next_card(deck_id: i64, state: State<AppState>) -> Result<CardInfo, String> {
    let collection = state.collection.lock().map_err(|_| "Failed to lock collection")?;
    let collection = collection.as_ref().ok_or("Collection not initialized")?;
    
    // Query for the next card in the deck
    let conn = collection.storage.db();
    
    // Find a due card in the deck
    let result: Result<(i64, i64), _> = conn.query_row(
        "SELECT id, nid FROM cards WHERE did = ? AND (queue > 0 OR queue < 0) LIMIT 1",
        params![deck_id],
        |row| Ok((row.get(0)?, row.get(1)?))
    );
    
    let (card_id, note_id) = result.map_err(|_| "No cards left to study")?;
    
    // Get the note to extract front/back fields
    let flds_json: String = conn.query_row(
        "SELECT flds FROM notes WHERE id = ?",
        params![note_id],
        |row| row.get(0)
    ).map_err(|e| format!("Note {} not found: {}", note_id, e))?;
    
    // Parse fields from JSON
    let fields: Vec<String> = serde_json::from_str(&flds_json)
        .map_err(|e| format!("Failed to parse note fields: {}", e))?;
    
    let front = fields.get(0).cloned().unwrap_or_default();
    let back = fields.get(1).cloned().unwrap_or_default();
    
    // Get the card flags
    let flags: i32 = conn.query_row(
        "SELECT flags FROM cards WHERE id = ?",
        params![card_id],
        |row| row.get(0)
    ).unwrap_or(0);
    let flag = (flags & 0xFF) as u8; // Lower 8 bits contain the flag value
    
    Ok(CardInfo {
        card_id,
        note_id,
        front,
        back,
        flag,
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

