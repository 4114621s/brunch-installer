// =======================================================
// installer.rs — לוגיקת ההתקנה הראשית
//
// זהו הקובץ הכי קריטי לביצועים:
// קובץ של 20GB+ חייב להיות מועתק עם Buffered Streams,
// אחרת ה-UI יקפא ו-Windows יציג "לא מגיב".
// =======================================================

use crate::{grub, windows_utils, ProgressEvent};
use std::io::{BufReader, BufWriter, Read, Write};
use std::time::Instant;
use tauri::{AppHandle, Emitter};

// ───────────────────────────────────────────────────────
// קבועים (Constants)
// ───────────────────────────────────────────────────────

/// גודל ה-Buffer לקריאה וכתיבה — 8 מגה-בייט
/// מאזן בין מהירות (גדול יותר = מהיר יותר) לשימוש בזיכרון
const BUFFER_SIZE: usize = 8 * 1024 * 1024; // 8 MB

/// כל כמה בייטים לעדכן את ה-UI
/// 64MB בין עדכונים = ~400ms על דיסק מהיר
/// מניעת "הצפת" אירועים ל-UI thread
const PROGRESS_REPORT_INTERVAL: u64 = 64 * 1024 * 1024; // 64 MB

// ───────────────────────────────────────────────────────
// פונקציה ראשית — async wrapper
// ───────────────────────────────────────────────────────

/// מריץ את כל שלבי ההתקנה ברצף
///
/// ⚠️ חשוב: קריאות I/O חוסמות (blocking) רצות בתוך
/// tokio::task::spawn_blocking כדי לא לחסום את ה-Tokio runtime.
/// ללא זה, ה-UI יקפא לחלוטין בזמן ההעתקה.
pub async fn run_installation(
    app: AppHandle,
    img_path: String,
    dest_folder: String,
    img_filename: String,
) -> Result<(), String> {
    // ── שלב 1: יצירת תיקיית יעד ──────────────────────
    emit(&app, 0.0, 0, 0, 0.0, "יוצר תיקיית יעד...");

    std::fs::create_dir_all(&dest_folder)
        .map_err(|e| format!("לא ניתן ליצור תיקייה '{}': {}", dest_folder, e))?;

    // ── שלב 2: השבתת Fast Startup ────────────────────
    emit(&app, 1.0, 0, 0, 0.0, "משבית Windows Fast Startup...");

    // לא קריטי — מדווח על שגיאה אבל ממשיך
    if let Err(e) = windows_utils::disable_fast_startup() {
        eprintln!("⚠️ אזהרה: לא הצלחנו להשבית Fast Startup: {}", e);
    }

    // ── שלב 3: בניית נתיב היעד ───────────────────────
    // Windows: backslash, לא forward-slash
    let dest_img_path = format!("{}\\{}", dest_folder.trim_end_matches('\\'), img_filename);

    // ── שלב 4: העתקת הקובץ ─────────────────────────
    // ⚠️ זה ה-blocking I/O — חייב לרוץ ב-thread pool נפרד
    // spawn_blocking מוציא את הלולאה מה-Tokio runtime
    // כדי שהאירועים של ה-UI ימשיכו לזרום
    let app_clone = app.clone();
    let img_path_clone = img_path.clone();
    let dest_img_clone = dest_img_path.clone();

    let copy_result = tokio::task::spawn_blocking(move || {
        copy_with_progress_sync(&app_clone, &img_path_clone, &dest_img_clone)
    })
    .await;

    // טיפול בשתי רמות השגיאה:
    // 1. JoinError (ה-thread קרס) 
    // 2. שגיאת העתקה עצמה
    match copy_result {
        Ok(Ok(())) => {} // הצלחה מושלמת
        Ok(Err(e)) => return Err(e), // שגיאת I/O
        Err(e) => return Err(format!("שגיאה קריטית ב-thread ההעתקה: {}", e)),
    }

    // ── שלב 5: כתיבת ערך GRUB ──────────────────────
    emit(&app, 99.0, 0, 0, 0.0, "כותב ערך GRUB2...");

    grub::write_grub_entry(&dest_folder, &img_filename)
        .map_err(|e| format!("שגיאה בכתיבת ערך GRUB: {}", e))?;

    // ── שלב 6: סיום ─────────────────────────────────
    emit(&app, 100.0, 0, 0, 0.0, "ההתקנה הושלמה בהצלחה! 🎉");

    Ok(())
}

// ───────────────────────────────────────────────────────
// לב ההתקנה: העתקה עם Buffered Streams
// ───────────────────────────────────────────────────────

/// מעתיקה קובץ גדול (20GB+) בצורה יעילה
///
/// למה Buffered Streams?
/// - ללא buffer: כל write() קורא syscall → הדיסק מקבל מיליוני בקשות קטנות → איטי מאוד
/// - עם buffer 8MB: מצבר 8MB בזיכרון → כותב ל-disk בפעימות גדולות → מהיר × 10
/// - BufWriter גם מונע שה-UI thread יצטרך לחכות לכל כתיבה
fn copy_with_progress_sync(
    app: &AppHandle,
    src_path: &str,
    dest_path: &str,
) -> Result<(), String> {
    // בדיקת גודל המקור
    let total_bytes = std::fs::metadata(src_path)
        .map(|m| m.len())
        .map_err(|e| format!("לא נמצא קובץ מקור '{}': {}", src_path, e))?;

    if total_bytes == 0 {
        return Err("קובץ המקור ריק — בדוק שבחרת את הקובץ הנכון".to_string());
    }

    // פתיחת קובץ מקור
    let src_file = std::fs::File::open(src_path)
        .map_err(|e| format!("לא ניתן לפתוח קובץ מקור: {}", e))?;

    // יצירת קובץ יעד
    let dest_file = std::fs::File::create(dest_path).map_err(|e| {
        format!(
            "לא ניתן ליצור קובץ יעד '{}': {} — בדוק שיש מספיק מקום",
            dest_path, e
        )
    })?;

    // ── Buffered Reader ──
    // קורא מהדיסק בחתיכות של 8MB
    // מונע קריאות disk קטנות ומרובות
    let mut reader = BufReader::with_capacity(BUFFER_SIZE, src_file);

    // ── Buffered Writer ──
    // אוגר 8MB בזיכרון לפני כתיבה לדיסק
    // המפתח למניעת קריסת UI בקבצים גדולים
    let mut writer = BufWriter::with_capacity(BUFFER_SIZE, dest_file);

    // מאגר זמני בזיכרון לנתוני ה-chunk הנוכחי
    let mut buffer = vec![0u8; BUFFER_SIZE];

    let mut bytes_copied: u64 = 0;
    let mut last_report_at: u64 = 0;
    let start_time = Instant::now();

    // ── לולאת ההעתקה ──
    loop {
        // קריאת chunk אחד מהמקור (עד 8MB)
        let bytes_read = reader
            .read(&mut buffer)
            .map_err(|e| format!("שגיאה בקריאה מהקובץ (byte {}): {}", bytes_copied, e))?;

        // bytes_read == 0 = הגענו לסוף הקובץ
        if bytes_read == 0 {
            break;
        }

        // כתיבת ה-chunk ליעד
        writer
            .write_all(&buffer[..bytes_read])
            .map_err(|e| format!("שגיאה בכתיבה לדיסק (byte {}): {}", bytes_copied, e))?;

        bytes_copied += bytes_read as u64;

        // דיווח ל-UI רק כל PROGRESS_REPORT_INTERVAL בייטים
        // (או בבייט האחרון)
        let should_report = bytes_copied - last_report_at >= PROGRESS_REPORT_INTERVAL
            || bytes_copied == total_bytes;

        if should_report {
            let elapsed = start_time.elapsed().as_secs_f64();
            let speed_mbps = if elapsed > 0.001 {
                (bytes_copied as f64 / 1_048_576.0) / elapsed
            } else {
                0.0
            };

            // 0–97% לשלב ההעתקה (3% נשמרים לGRUB + סיום)
            let percentage = (bytes_copied as f64 / total_bytes as f64) * 97.0;

            let status = format!(
                "מעתיק... {:.2} GB מתוך {:.2} GB — {:.1} MB/s",
                bytes_copied as f64 / 1_073_741_824.0,
                total_bytes as f64 / 1_073_741_824.0,
                speed_mbps
            );

            emit(app, percentage, bytes_copied, total_bytes, speed_mbps, &status);
            last_report_at = bytes_copied;
        }
    }

    // ── חשוב מאוד: flush! ──
    // BufWriter שומר עד 8MB בזיכרון.
    // ללא flush, הבייטים האחרונים לא ייכתבו לדיסק!
    writer
        .flush()
        .map_err(|e| format!("שגיאה בכתיבה הסופית לדיסק (flush): {}", e))?;

    Ok(())
}

// ───────────────────────────────────────────────────────
// פונקציית עזר לשליחת אירועי progress
// ───────────────────────────────────────────────────────

/// שולחת אירוע "installation-progress" ל-TypeScript
/// הטיפוסים חייבים להתאים ל-ProgressEvent ב-TypeScript
fn emit(
    app: &AppHandle,
    percentage: f64,
    bytes_copied: u64,
    total_bytes: u64,
    speed_mbps: f64,
    status: &str,
) {
    // נתעלם משגיאות emit — ה-UI אולי נסגר, לא קריטי
    let _ = app.emit(
        "installation-progress",
        ProgressEvent {
            bytes_copied,
            total_bytes,
            percentage,
            speed_mbps,
            status: status.to_string(),
        },
    );
}
