// =======================================================
// windows_utils.rs — כלים ייחודיים ל-Windows
//
// מכיל:
// 1. בדיקת BitLocker
// 2. השבתת Fast Startup (כתיבה ל-Registry)
// 3. רשימת כוננים זמינים
// 4. בדיקת הרשאות Admin
//
// ⚠️ כל הפונקציות מוגנות ב-#[cfg(windows)] —
//    גרסאות stub ל-Linux/Mac קיימות לצורך פיתוח
// =======================================================

use crate::DriveInfo;
use std::process::Command;

#[cfg(windows)]
use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

// ───────────────────────────────────────────────────────
// בדיקת BitLocker
// ───────────────────────────────────────────────────────

/// בודק אם BitLocker פעיל על כונן נתון
///
/// drive_letter: "C" (ללא נקודתיים)
/// מחזיר: true = BitLocker פעיל (⚠️ סכנה!), false = בטוח
///
/// ⚠️ מדוע זה קריטי:
/// אם BitLocker פעיל, ה-GRUB לא יוכל לקרוא את ה-NTFS partition.
/// המשתמש יקבל "error: disk read failed" בעת אתחול ChromeOS.
pub fn is_bitlocker_enabled(drive_letter: &str) -> Result<bool, String> {
    let letter = drive_letter
        .trim()
        .trim_end_matches(':')
        .to_uppercase();
    let drive_with_colon = format!("{}:", letter);

    // שאילת PowerShell על סטטוס BitLocker
    // ErrorAction Stop — זורק exception אם הכונן לא נמצא
    // catch {} — מחזיר DISABLED אם אין BitLocker/שגיאה
    let script = format!(
        r#"try {{
            $v = Get-BitLockerVolume -MountPoint '{drive}' -ErrorAction Stop
            if ($v.ProtectionStatus -eq 'On') {{ 'ENABLED' }} else {{ 'DISABLED' }}
        }} catch {{ 'DISABLED' }}"#,
        drive = drive_with_colon
    );

    let output = run_powershell(&script)?;
    Ok(output.trim().to_uppercase().contains("ENABLED"))
}

// ───────────────────────────────────────────────────────
// השבתת Fast Startup
// ───────────────────────────────────────────────────────

/// משבית את Windows Fast Startup דרך ה-Registry
///
/// מפתח Registry:
/// HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Power
/// ערך: HiberbootEnabled = 0 (REG_DWORD)
///
/// ⚠️ מדוע זה קריטי:
/// Fast Startup שומר snapshot של Windows kernel בקובץ hiberfil.sys
/// ומשאיר את ה-NTFS partition במצב "dirty" (לא unmounted כראוי).
/// ChromeOS לא יוכל לכתוב לדיסק — כל שינוי יאבד!
///
/// ⚠️ נדרשות הרשאות Administrator לכתיבה ל-HKLM
#[cfg(windows)]
pub fn disable_fast_startup() -> Result<(), String> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    // פתיחת מפתח Power (create_subkey פותח עם WRITE access כברירת מחדל)
    let (power_key, _) = hklm
        .create_subkey(r"SYSTEM\CurrentControlSet\Control\Session Manager\Power")
        .map_err(|e| {
            format!(
                "אין גישה ל-Registry (נדרש Administrator): {}",
                e
            )
        })?;

    // כתיבת 0 = כיבוי HiberBoot (Fast Startup)
    power_key
        .set_value("HiberbootEnabled", &0u32)
        .map_err(|e| format!("שגיאה בכתיבה ל-Registry: {}", e))?;

    Ok(())
}

// גרסת stub ל-Linux/Mac (לפיתוח בלבד)
#[cfg(not(windows))]
pub fn disable_fast_startup() -> Result<(), String> {
    println!("[DEV] disable_fast_startup: stub on non-Windows");
    Ok(())
}

// ───────────────────────────────────────────────────────
// רשימת כוננים
// ───────────────────────────────────────────────────────

/// מחזיר רשימה של כל הכוננים הזמינים עם פרטיהם
///
/// משתמש בפקודה PowerShell אחת (יעיל יותר מלולאה על A-Z)
pub fn list_drives() -> Result<Vec<DriveInfo>, String> {
    #[cfg(windows)]
    {
        // שאילת כל הכוננים בפקודה אחת ויצוא כ-JSON
        // Where-Object: רק כוננים עם אות (לא כוננים ללא mount point)
        // Select-Object: בחירת שדות רלוונטיים עם שמות קצרים
        let script = r#"
Get-Volume |
  Where-Object { $_.DriveLetter -ne $null -and $_.DriveLetter -ne '' } |
  Select-Object
    @{N='Letter'; E={[string]$_.DriveLetter}},
    @{N='Label';  E={if($_.FileSystemLabel){$_.FileSystemLabel}else{''}}},
    @{N='FS';     E={if($_.FileSystem){$_.FileSystem}else{'Unknown'}}},
    @{N='Free';   E={[int64]$_.SizeRemaining}},
    @{N='Total';  E={[int64]$_.Size}} |
  ConvertTo-Json -Compress
"#
        .split('\n')
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join(" ");

        let json_output = run_powershell(&script)?;

        if json_output.trim().is_empty() {
            return Ok(vec![]);
        }

        // PowerShell מחזיר {} (אובייקט) כשיש כונן אחד,
        // או [{},{}] (מערך) כשיש מספר כוננים
        let parsed: serde_json::Value = serde_json::from_str(json_output.trim())
            .map_err(|e| format!("שגיאה בפענוח JSON מ-PowerShell: {}", e))?;

        let items: Vec<serde_json::Value> = match parsed {
            serde_json::Value::Array(arr) => arr,
            obj @ serde_json::Value::Object(_) => vec![obj],
            _ => return Ok(vec![]),
        };

        let mut drives = Vec::new();
        for item in items {
            let letter = match item["Letter"].as_str() {
                Some(l) if !l.is_empty() && l != "null" => l.to_string(),
                _ => continue,
            };

            let raw_label = item["Label"].as_str().unwrap_or("").to_string();
            let display_label = if raw_label.is_empty() {
                format!("כונן {}:", letter)
            } else {
                format!("{} ({}:)", raw_label, letter)
            };

            drives.push(DriveInfo {
                letter,
                label: display_label,
                free_bytes: item["Free"].as_i64().unwrap_or(0).max(0) as u64,
                total_bytes: item["Total"].as_i64().unwrap_or(0).max(0) as u64,
                filesystem: item["FS"].as_str().unwrap_or("Unknown").to_string(),
            });
        }

        // מיון לפי אות הכונן
        drives.sort_by(|a, b| a.letter.cmp(&b.letter));
        Ok(drives)
    }

    // גרסת stub ל-Linux/Mac לצורך פיתוח
    #[cfg(not(windows))]
    {
        Ok(vec![
            DriveInfo {
                letter: "C".to_string(),
                label: "Windows (C:) [DEV STUB]".to_string(),
                free_bytes: 230 * 1024 * 1024 * 1024,
                total_bytes: 465 * 1024 * 1024 * 1024,
                filesystem: "NTFS".to_string(),
            },
            DriveInfo {
                letter: "D".to_string(),
                label: "Data (D:) [DEV STUB]".to_string(),
                free_bytes: 450 * 1024 * 1024 * 1024,
                total_bytes: 931 * 1024 * 1024 * 1024,
                filesystem: "NTFS".to_string(),
            },
        ])
    }
}

// ───────────────────────────────────────────────────────
// בדיקת הרשאות Admin
// ───────────────────────────────────────────────────────

/// בודק אם האפליקציה רצה עם הרשאות Administrator
///
/// פקודת "net session" מצליחה רק עם Admin rights
pub fn is_running_as_admin() -> Result<bool, String> {
    #[cfg(windows)]
    {
        let output = Command::new("net")
            .arg("session")
            .output()
            .map_err(|e| format!("שגיאה בבדיקת הרשאות: {}", e))?;
        Ok(output.status.success())
    }

    #[cfg(not(windows))]
    {
        Ok(true) // ב-Linux/Mac תמיד מאשר (לצורך פיתוח)
    }
}

// ───────────────────────────────────────────────────────
// פונקציית עזר: הרצת PowerShell
// ───────────────────────────────────────────────────────

/// מריץ פקודת PowerShell ומחזיר את הפלט כטקסט
///
/// דגלים:
/// -NoProfile       — מהירות הפעלה (ללא טעינת פרופיל)
/// -NonInteractive  — ללא שאלות אינטראקטיביות
/// -Command         — הפקודה לביצוע
fn run_powershell(script: &str) -> Result<String, String> {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .map_err(|e| format!("לא ניתן להריץ PowerShell: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // לא כל שגיאה ב-stderr היא קריטית (למשל, אזהרות)
        // נחזיר את ה-stdout בכל מקרה
        eprintln!("PowerShell stderr: {}", stderr);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
