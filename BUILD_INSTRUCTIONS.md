# הוראות בנייה — Brunch / ChromeOS Installer

## דרישות מוקדמות

התקן את הכלים הבאים **לפי הסדר**:

### 1. Node.js (v18 ומעלה)
- הורד מ: https://nodejs.org/en/download
- וודא עם: `node --version`

### 2. pnpm (מנהל חבילות JavaScript)
```
npm install -g pnpm
```
- וודא עם: `pnpm --version`

### 3. Rust
- הורד מ: https://rustup.rs (לחץ על הקישור וסרוק)
- הפעל את `rustup-init.exe` ולחץ Enter
- וודא עם: `rustc --version`

### 4. Visual Studio Build Tools (Windows בלבד)
- הורד מ: https://visualstudio.microsoft.com/visual-cpp-build-tools/
- בהתקנה, בחר: **"Desktop development with C++"**
- זהו קומפיילר C++ שנדרש ל-Tauri

### 5. WebView2 (Windows 11: כבר מותקן. Windows 10: יש להתקין)
- אם חסר, Tauri יציג שגיאה בזמן ריצה
- הורד מ: https://developer.microsoft.com/en-us/microsoft-edge/webview2/

---

## בנייה

```bash
# 1. כנס לתיקיית הפרויקט
cd brunch-installer

# 2. התקן תלויות JavaScript
pnpm install

# 3. בנה את קובץ ה-EXE הסופי
pnpm tauri build
```

### תוצאה:
```
src-tauri/target/release/bundle/nsis/
  brunch-installer_1.0.0_x64-setup.exe   ← זה קובץ ההתקנה!
```

---

## פיתוח (Dev Mode)

```bash
# מפעיל את האפליקציה עם Hot Reload
pnpm tauri dev
```

- כל שינוי ב-TypeScript → מתעדכן מיידית
- שינוי ב-Rust → מקמפל מחדש (לוקח ~30 שניות בפעם הראשונה)

---

## מבנה הפרויקט

```
brunch-installer/
│
├── src-tauri/                  ← Backend (Rust)
│   ├── src/
│   │   ├── main.rs             ← נקודת כניסה (שורה 1)
│   │   ├── lib.rs              ← פקודות Tauri
│   │   ├── installer.rs        ← העתקה + Progress Bar
│   │   ├── grub.rs             ← יצירת ערך GRUB2
│   │   └── windows_utils.rs    ← BitLocker, Registry, כוננים
│   ├── Cargo.toml              ← תלויות Rust
│   ├── build.rs                ← סקריפט בנייה
│   ├── tauri.conf.json         ← הגדרות Tauri
│   └── capabilities/
│       └── default.json        ← הרשאות Tauri v2
│
├── src/                        ← Frontend (TypeScript)
│   ├── app.ts                  ← לוגיקת UI מלאה
│   └── styles.css              ← עיצוב dark theme
│
├── index.html                  ← מסך ראשי
├── package.json                ← תלויות JS
├── vite.config.js              ← הגדרות Vite
└── tsconfig.json               ← הגדרות TypeScript
```

---

## פתרון בעיות נפוצות

### שגיאה: `error: linker 'link.exe' not found`
→ VS Build Tools לא מותקן, או נדרש restart לאחר ההתקנה.

### שגיאה: `Could not find WebView2`
→ התקן WebView2 Runtime מ-Microsoft.

### שגיאה: `tauri-build` גרסה לא תואמת
→ הרץ `pnpm install` שוב אחרי `cargo update` בתיקיית src-tauri.

### שגיאה ב-PowerShell: `Get-Volume is not recognized`
→ זה נורמלי על Windows 7/8. הפרויקט מיועד ל-Windows 10/11 בלבד.

---

## זרימת ההתקנה (לעיון)

```
משתמש בוחר כונן
      ↓
בדיקת BitLocker (אזהרה בלבד)
      ↓
משתמש בוחר קובץ .img/.bin
      ↓
START INSTALLATION
      ├── create_dir_all(C:\brunch)
      ├── disable_fast_startup() → Registry HiberbootEnabled=0
      ├── copy_file() ← spawn_blocking + BufReader/BufWriter 8MB
      │      emit("installation-progress") כל 64MB
      └── write_grub_entry() → C:\brunch\grub_entry.txt
            ↓
משתמש מוסיף grub_entry.txt ל-grub.cfg
      ↓
🎉 ChromeOS עולה בתפריט GRUB
```
