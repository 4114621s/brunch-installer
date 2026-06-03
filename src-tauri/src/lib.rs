// =======================================================
// lib.rs — ה-Backend המרכזי ב-Rust
// רישום כל הפקודות שה-Frontend (TypeScript) קורא להן
// =======================================================

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

// ייבוא המודולים הפנימיים שלנו
mod grub;
mod installer;
mod windows_utils;

// ─────────────────────────────────────────────
// מבני נתונים משותפים (Shared Data Structures)
// ─────────────────────────────────────────────

/// מידע על כונן אחד — נשלח ל-Frontend
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DriveInfo {
    pub letter: String,     // "C"
    pub label: String,      // שם תצוגה: "Windows (C:)"
    pub free_bytes: u64,    // מקום פנוי בבייטים
    pub total_bytes: u64,   // גודל כולל
    pub filesystem: String, // "NTFS", "FAT32", "exFAT"...
}

/// אירוע התקדמות — נפלט ל-Frontend כל ~64MB בזמן העתקה
/// TypeScript מקשיב לאירוע "installation-progress"
#[derive(Clone, Serialize)]
pub struct ProgressEvent {
    pub bytes_copied: u64, // כמה בייטים הועתקו עד כה
    pub total_bytes: u64,  // גודל הקובץ הכולל
    pub percentage: f64,   // אחוז (0.0–100.0)
    pub speed_mbps: f64,   // מהירות בMB/s
    pub status: String,    // הודעת סטטוס למשתמש
}

// ─────────────────────────────────────────────
// פקודות Tauri — נחשפות ל-TypeScript
// ─────────────────────────────────────────────

/// בדיקה אם התוכנית רצה עם הרשאות Administrator
/// נדרש לכתיבה ל-Registry ולגישה ל-EFI
#[tauri::command]
async fn check_admin_rights() -> Result<bool, String> {
    windows_utils::is_running_as_admin()
        .map_err(|e| format!("שגיאה בבדיקת הרשאות: {}", e))
}

/// קבלת רשימת הכוננים הזמינים
#[tauri::command]
async fn get_available_drives() -> Result<Vec<DriveInfo>, String> {
    windows_utils::list_drives()
        .map_err(|e| format!("שגיאה בקריאת כוננים: {}", e))
}

/// בדיקה אם BitLocker מופעל על כונן מסוים
/// drive_letter: "C" (ללא נקודתיים)
///
/// ⚠️ סכנה: BitLocker פעיל = GRUB לא יוכל לקרוא NTFS
#[tauri::command]
async fn check_bitlocker(drive_letter: String) -> Result<bool, String> {
    windows_utils::is_bitlocker_enabled(&drive_letter)
        .map_err(|e| format!("שגיאה בבדיקת BitLocker: {}", e))
}

/// השבתת Fast Startup דרך ה-Registry
///
/// ⚠️ חובה! Fast Startup גורם ל-NTFS להישאר "נעול"
/// ChromeOS לא יוכל לכתוב לדיסק בלי השבתה זו
#[tauri::command]
async fn disable_fast_startup() -> Result<(), String> {
    windows_utils::disable_fast_startup()
        .map_err(|e| format!("שגיאה בהשבתת Fast Startup: {}", e))
}

/// קבלת גודל קובץ בבייטים
#[tauri::command]
async fn get_file_size(path: String) -> Result<u64, String> {
    std::fs::metadata(&path)
        .map(|m| m.len())
        .map_err(|e| format!("לא נמצא קובץ '{}': {}", path, e))
}

/// הפקודה הראשית: מריץ את כל תהליך ההתקנה
///
/// app:          handle לאפליקציה — לשליחת אירועי progress
/// img_path:     נתיב לקובץ ה-.img/.bin שנבחר
/// dest_folder:  תיקיית יעד (למשל "C:\\brunch")
/// img_filename: שם הקובץ שישמר (למשל "chromeos.img")
#[tauri::command]
async fn start_installation(
    app: AppHandle,
    img_path: String,
    dest_folder: String,
    img_filename: String,
) -> Result<(), String> {
    installer::run_installation(app, img_path, dest_folder, img_filename).await
}

/// מחזיר את תוכן ערך ה-GRUB שנוצר (להצגה למשתמש)
#[tauri::command]
async fn get_grub_entry_content(
    img_filename: String,
    dest_folder: String,
) -> Result<String, String> {
    Ok(grub::generate_grub_entry(&img_filename, &dest_folder))
}

// ─────────────────────────────────────────────
// הפעלת האפליקציה
// ─────────────────────────────────────────────

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            check_admin_rights,
            get_available_drives,
            check_bitlocker,
            disable_fast_startup,
            get_file_size,
            start_installation,
            get_grub_entry_content,
        ])
        .run(tauri::generate_context!())
        .expect("שגיאה קריטית בהפעלת האפליקציה");
}
