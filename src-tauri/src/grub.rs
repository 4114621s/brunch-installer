// =======================================================
// grub.rs — יצירת ערך GRUB2 לאתחול Brunch / ChromeOS
//
// ערך GRUB2 תקני ל-Brunch Framework:
// - משתמש במודולי ntfs ו-loopback
// - מאתר את הכונן אוטומטית (search --set=root)
// - טוען את גרעין ChromeOS מתוך האימג'
// =======================================================

use std::fs;
use std::io::Write;
use std::path::Path;

// ───────────────────────────────────────────────────────
// יצירת תוכן ערך ה-GRUB
// ───────────────────────────────────────────────────────

/// מייצר מחרוזת מלאה של ערך GRUB2 עבור Brunch
///
/// img_filename: שם קובץ האימג' (למשל "chromeos.img")
/// _dest_folder: תיקיית ההתקנה — בשימוש עתידי (RFU)
///
/// הנחה: הקובץ מותקן תחת /brunch/<img_filename>
/// (בשורש הכונן שנבחר)
pub fn generate_grub_entry(img_filename: &str, _dest_folder: &str) -> String {
    // נתיב הקובץ כפי שGRUB רואה אותו (forward slash, לא backslash)
    let grub_img_path = format!("/brunch/{}", img_filename);

    // הסבר על המבנה:
    // (loop,7) = מחיצה מספר 7 בתוך האימג' = ChromeOS STATE partition
    //            Brunch תמיד שומר את STATE ב-partition 7
    // loop.max_part=16 = GRUB יזהה את כל 16 המחיצות של האימג'
    // img_part=$root = איפה נמצאת המחיצה שמכילה את קובץ האימג'
    //                  $root מוגדר ע"י ה-search למעלה

    format!(
        r#"# ===================================================
# ערך GRUB2 — Brunch Framework / ChromeOS
# נוצר אוטומטית ע"י Brunch Installer
#
# הוראות שימוש:
# 1. פתח את /boot/grub/grub.cfg (או /boot/grub2/grub.cfg)
# 2. הוסף את הבלוק הזה בסוף הקובץ
# 3. שמור וצא
# 4. אתחל — ChromeOS יופיע בתפריט GRUB
# ===================================================

menuentry "ChromeOS (Brunch)" --class "chromeos" {{

    # ── טעינת מודולים נדרשים ──
    # ntfs: תמיכה בקריאת כונן NTFS (Windows)
    # loopback: הרכבת קובץ img כהתקן דיסק
    insmod part_gpt
    insmod part_msdos
    insmod ntfs
    insmod loopback
    insmod linux

    # ── מציאת הכונן ──
    # GRUB יחפש בכל הכוננים את זה שמכיל את קובץ האימג'
    # ומגדיר אותו כ-$root (לא נדרש לדעת מראש C: = hd0,gpt2)
    set img_path={img_path}
    search --no-floppy --set=root --file $img_path

    # ── הרכבת האימג' כדיסק לולאה ──
    # אחרי שורה זו, (loop,N) מפנה למחיצות בתוך האימג'
    loopback loop $img_path

    # ── טעינת גרעין ChromeOS ──
    # (loop,7) = מחיצה 7 בתוך האימג' = ChromeOS STATE
    # הפרמטרים:
    #   boot=local          — אתחול ממחיצה מקומית (לא USB)
    #   noresume            — דילוג על resume from hibernate
    #   noswap              — ללא swap (ChromeOS לא צריך)
    #   loglevel=7          — לוגים מפורטים לאבחון
    #   disablevmx=off      — הפעלת VT-x (וירטואליזציה)
    #   cros_secure         — אבטחת ChromeOS
    #   cros_debug          — מצב debug
    #   loop.max_part=16    — זיהוי כל 16 מחיצות האימג'
    #   img_part=$root      — כונן שמכיל את האימג' (מה-search)
    #   img_path=...        — נתיב מלא לקובץ האימג'
    linux (loop,7)/kernel boot=local noresume noswap loglevel=7 \
          disablevmx=off cros_secure cros_debug \
          options= loop.max_part=16 \
          img_part=$root img_path=$img_path \
          console= vt.global_cursor_default=0 \
          brunch_bootsplash=default quiet

    # ── טעינת initramfs ועדכוני מיקרוקוד ──
    # amd-ucode.img  — עדכוני מיקרוקוד AMD (לא פוגע אם אין)
    # intel-ucode.img — עדכוני מיקרוקוד Intel (לא פוגע אם אין)
    # initramfs.img  — Initial RAM Filesystem (חובה)
    initrd (loop,7)/lib/firmware/amd-ucode.img \
           (loop,7)/lib/firmware/intel-ucode.img \
           (loop,7)/initramfs.img
}}
"#,
        img_path = grub_img_path
    )
}

// ───────────────────────────────────────────────────────
// כתיבת ערך ה-GRUB לדיסק
// ───────────────────────────────────────────────────────

/// כותב את ערך ה-GRUB לקובץ grub_entry.txt בתיקיית ההתקנה
///
/// dest_folder:  למשל "C:\\brunch"
/// img_filename: למשל "chromeos.img"
///
/// תוצאה: C:\brunch\grub_entry.txt — המשתמש מעתיק את תוכנו ל-grub.cfg
pub fn write_grub_entry(dest_folder: &str, img_filename: &str) -> Result<(), String> {
    let content = generate_grub_entry(img_filename, dest_folder);

    // בניית נתיב לקובץ grub_entry.txt
    let entry_path = Path::new(dest_folder).join("grub_entry.txt");

    // יצירת הקובץ (דורס אם קיים)
    let mut file = fs::File::create(&entry_path).map_err(|e| {
        format!(
            "לא ניתן ליצור קובץ ב-'{}': {}",
            entry_path.display(),
            e
        )
    })?;

    file.write_all(content.as_bytes()).map_err(|e| {
        format!(
            "שגיאה בכתיבה ל-'{}': {}",
            entry_path.display(),
            e
        )
    })?;

    Ok(())
}
