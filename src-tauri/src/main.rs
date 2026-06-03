// =======================================================
// main.rs — נקודת הכניסה של האפליקציה
// =======================================================

// מניעת חלון שורת הפקודה השחורה ב-Windows
// (פעיל רק ב-Release build, לא ב-Dev)
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    brunch_installer_lib::run()
}
