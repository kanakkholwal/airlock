use super::store::{ChatStore, Conversation, Message, Part, Role};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

pub struct ChatDb(pub Mutex<ChatStore>);

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
#[specta::specta]
pub fn chat_list_conversations(db: State<'_, ChatDb>) -> Result<Vec<Conversation>, String> {
    db.0.lock().unwrap().list_conversations().map_err(err)
}

#[tauri::command]
#[specta::specta]
pub fn chat_create_conversation(
    db: State<'_, ChatDb>,
    title: String,
    model: Option<String>,
) -> Result<i64, String> {
    db.0.lock()
        .unwrap()
        .create_conversation(&title, model.as_deref())
        .map_err(err)
}

#[tauri::command]
#[specta::specta]
pub fn chat_load_messages(
    db: State<'_, ChatDb>,
    conversation_id: i64,
) -> Result<Vec<Message>, String> {
    db.0.lock()
        .unwrap()
        .load_messages(conversation_id)
        .map_err(err)
}

#[tauri::command]
#[specta::specta]
pub fn chat_append_message(
    db: State<'_, ChatDb>,
    conversation_id: i64,
    role: Role,
    parts: Vec<Part>,
) -> Result<i64, String> {
    db.0.lock()
        .unwrap()
        .append_message(conversation_id, role, &parts)
        .map_err(err)
}

#[tauri::command]
#[specta::specta]
pub fn chat_delete_last_message(db: State<'_, ChatDb>, conversation_id: i64) -> Result<(), String> {
    db.0.lock()
        .unwrap()
        .delete_last_message(conversation_id)
        .map_err(err)
}

#[tauri::command]
#[specta::specta]
pub fn chat_rename_conversation(
    db: State<'_, ChatDb>,
    id: i64,
    title: String,
) -> Result<(), String> {
    db.0.lock()
        .unwrap()
        .rename_conversation(id, &title)
        .map_err(err)
}

#[tauri::command]
#[specta::specta]
pub fn chat_delete_conversation(db: State<'_, ChatDb>, id: i64) -> Result<(), String> {
    db.0.lock().unwrap().delete_conversation(id).map_err(err)
}

pub fn manage(app: &AppHandle) -> Result<(), String> {
    let dir = crate::portable::app_data_dir(app).map_err(err)?;
    std::fs::create_dir_all(&dir).map_err(err)?;
    let store = ChatStore::open(&dir.join("chat.db")).map_err(err)?;
    app.manage(ChatDb(Mutex::new(store)));
    Ok(())
}
