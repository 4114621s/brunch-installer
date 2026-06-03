// =======================================================
// app.ts — לוגיקת הממשק הגרפי של Brunch Installer
//
// ויזארד 5-שלבי:
//   1. בחירת כונן יעד
//   2. בחירת קובץ .img/.bin
//   3. אישור הגדרות
//   4. מסך התקנה + Progress Bar
//   5. מסך סיום + ערך GRUB
// =======================================================

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';

// ─────────────────────────────────────────────
// טיפוסי נתונים — חייבים להתאים ל-Rust!
// ─────────────────────────────────────────────

interface DriveInfo {
  letter: string;
  label: string;
  free_bytes: number;
  total_bytes: number;
  filesystem: string;
}

// חייב להתאים ל-ProgressEvent ב-lib.rs
interface ProgressPayload {
  bytes_copied: number;
  total_bytes: number;
  percentage: number;
  speed_mbps: number;
  status: string;
}

// ─────────────────────────────────────────────
// מצב האפליקציה (State Machine)
// ─────────────────────────────────────────────

const state = {
  step: 0,
  drives: [] as DriveInfo[],
  selectedDrive: '',
  selectedDriveInfo: null as DriveInfo | null,
  hasBitLocker: false,
  imgPath: '',
  imgFileName: '',
  imgSize: 0,
  destFolder: '',
  grubEntry: '',
  unlistenFn: null as UnlistenFn | null,
};

// ─────────────────────────────────────────────
// פונקציות עזר
// ─────────────────────────────────────────────

function formatBytes(bytes: number): string {
  if (bytes >= 1e12) return `${(bytes / 1e12).toFixed(1)} TB`;
  if (bytes >= 1e9)  return `${(bytes / 1e9).toFixed(1)} GB`;
  if (bytes >= 1e6)  return `${(bytes / 1e6).toFixed(1)} MB`;
  return `${bytes} B`;
}

function $(id: string): HTMLElement {
  return document.getElementById(id) as HTMLElement;
}

function html(id: string, content: string): void {
  $(id).innerHTML = content;
}

function escHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;').replace(/</g, '&lt;')
    .replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

function showScreen(id: string): void {
  document.querySelectorAll<HTMLElement>('.screen').forEach(el => {
    el.style.display = 'none';
  });
  $(id).style.display = 'flex';
}

// ─────────────────────────────────────────────
// אתחול — בדיקות ראשוניות
// ─────────────────────────────────────────────

async function init(): Promise<void> {
  showScreen('screen-loading');
  html('loading-msg', 'בודק הרשאות מנהל מערכת...');

  // בדיקת Admin
  let isAdmin = false;
  try {
    isAdmin = await invoke<boolean>('check_admin_rights');
  } catch {
    isAdmin = false;
  }

  if (!isAdmin) {
    showScreen('screen-no-admin');
    return;
  }

  html('loading-msg', 'טוען רשימת כוננים...');

  try {
    state.drives = await invoke<DriveInfo[]>('get_available_drives');
  } catch (e) {
    showError(`לא ניתן לטעון רשימת כוננים: ${e}`);
    return;
  }

  goToStep(1);
}

// ─────────────────────────────────────────────
// שלב 1: בחירת כונן יעד
// ─────────────────────────────────────────────

function renderStep1(): void {
  const drivesHtml = state.drives.map(d => {
    const isNTFS = d.filesystem.toUpperCase() === 'NTFS';
    const pct = d.total_bytes > 0
      ? ((d.total_bytes - d.free_bytes) / d.total_bytes * 100).toFixed(0)
      : '0';
    const fsClass = isNTFS ? 'success' : 'error';
    const dis = isNTFS ? '' : 'disabled';

    return `
      <label class="drive-option ${isNTFS ? '' : 'disabled'}">
        <input type="radio" name="drive" value="${d.letter}"
               ${dis}
               onchange="window._driveSelected('${d.letter}')">
        <div class="drive-info">
          <div class="drive-name">
            <strong>${escHtml(d.label)}</strong>
            <span class="badge ${fsClass}">${escHtml(d.filesystem)}</span>
            ${!isNTFS ? '<span class="badge error">לא נתמך</span>' : ''}
          </div>
          <div class="drive-space">
            <div class="space-bar">
              <div class="space-used" style="width:${pct}%"></div>
            </div>
            <span class="space-text">
              ${formatBytes(d.free_bytes)} פנוי מתוך ${formatBytes(d.total_bytes)}
            </span>
          </div>
        </div>
      </label>`;
  }).join('');

  html('step-content', `
    <div class="step-header">
      <h2>שלב 1 — בחירת כונן יעד</h2>
      <p class="subtitle">
        בחר את הכונן שבו יותקן ChromeOS.<br>
        הכונן חייב להיות <strong>NTFS</strong> עם לפחות 20 GB פנוי.
      </p>
    </div>

    <div class="drives-list">
      ${drivesHtml || '<p class="empty-state">לא נמצאו כוננים NTFS מחוברים</p>'}
    </div>

    <div id="bitlocker-warning" class="alert alert-danger" style="display:none">
      <strong>⚠️ BitLocker מופעל על כונן זה!</strong><br>
      BitLocker מונע מ-ChromeOS לגשת לדיסק. ה-GRUB לא יוכל לקרוא את הנתונים.<br>
      <strong>כבה את BitLocker לפני ההתקנה, אחרת המחשב לא יאתחל לחלוטין.</strong>
    </div>

    <div class="step-actions">
      <div></div>
      <button class="btn btn-primary" id="btn-next-1"
              onclick="window._nextStep()" disabled>
        הבא &larr;
      </button>
    </div>
  `);
}

async function driveSelected(letter: string): Promise<void> {
  state.selectedDrive = letter;
  state.selectedDriveInfo = state.drives.find(d => d.letter === letter) ?? null;
  state.destFolder = `${letter}:\\brunch`;

  // בדיקת BitLocker
  const warning = $('bitlocker-warning');
  try {
    state.hasBitLocker = await invoke<boolean>('check_bitlocker', {
      driveLetter: letter,
    });
    warning.style.display = state.hasBitLocker ? 'block' : 'none';
  } catch {
    warning.style.display = 'none';
    state.hasBitLocker = false;
  }

  ($('btn-next-1') as HTMLButtonElement).disabled = false;
}

// ─────────────────────────────────────────────
// שלב 2: בחירת קובץ Brunch
// ─────────────────────────────────────────────

function renderStep2(): void {
  html('step-content', `
    <div class="step-header">
      <h2>שלב 2 — בחירת קובץ Brunch</h2>
      <p class="subtitle">
        בחר את קובץ ה-<strong>.img</strong> או <strong>.bin</strong>
        שהורדת מ-Brunch Framework.<br>
        בדרך כלל גודלו בין 5 ל-30 GB.
      </p>
    </div>

    <div class="file-selector">
      <button class="btn btn-secondary btn-large"
              onclick="window._selectFile()">
        📂 &nbsp;בחר קובץ .img / .bin
      </button>
    </div>

    <div id="file-info" class="file-info" style="display:none">
      <div class="file-icon">📄</div>
      <div class="file-details">
        <div class="file-name" id="fn-name"></div>
        <div class="file-size" id="fn-size">מחשב...</div>
      </div>
      <div class="file-check">✓</div>
    </div>

    <div id="file-warning" class="alert alert-warning" style="display:none">
      ⚠️ הקובץ אינו מסוג .img או .bin — ייתכן שזה הקובץ הלא נכון.
    </div>

    <div class="step-actions">
      <button class="btn btn-ghost" onclick="window._prevStep()">
        &rarr; חזור
      </button>
      <button class="btn btn-primary" id="btn-next-2"
              onclick="window._nextStep()" disabled>
        הבא &larr;
      </button>
    </div>
  `);
}

async function selectFile(): Promise<void> {
  try {
    const selected = await open({
      multiple: false,
      filters: [
        { name: 'Brunch Image', extensions: ['img', 'bin'] },
        { name: 'All Files',    extensions: ['*'] },
      ],
    });

    if (!selected) return;

    state.imgPath = selected as string;

    // חילוץ שם הקובץ מהנתיב
    const parts = state.imgPath.replace(/\\/g, '/').split('/');
    state.imgFileName = parts[parts.length - 1] ?? 'chromeos.img';

    const ext = state.imgFileName.split('.').pop()?.toLowerCase() ?? '';
    const isValidExt = ext === 'img' || ext === 'bin';

    html('fn-name', escHtml(state.imgFileName));
    html('fn-size', 'מחשב גודל...');

    $('file-info').style.display    = 'flex';
    $('file-warning').style.display = isValidExt ? 'none' : 'block';

    // קבלת גודל הקובץ מ-Rust
    try {
      state.imgSize = await invoke<number>('get_file_size', { path: state.imgPath });
      html('fn-size', formatBytes(state.imgSize));
    } catch {
      html('fn-size', 'גודל לא ידוע');
    }

    ($('btn-next-2') as HTMLButtonElement).disabled = false;

  } catch (e) {
    console.error('File selection error:', e);
  }
}

// ─────────────────────────────────────────────
// שלב 3: אישור הגדרות
// ─────────────────────────────────────────────

function renderStep3(): void {
  const estMin = state.imgSize > 0
    ? Math.ceil(state.imgSize / (150 * 1024 * 1024) / 60)
    : '?';

  const blWarning = state.hasBitLocker ? `
    <div class="alert alert-danger">
      <strong>⚠️ BitLocker פעיל!</strong>
      ההתקנה עשויה להצליח, אך ChromeOS לא יאתחל עד לכיבוי BitLocker.
    </div>` : '';

  html('step-content', `
    <div class="step-header">
      <h2>שלב 3 — אישור הגדרות</h2>
      <p class="subtitle">בדוק את הפרטים לפני תחילת ההתקנה.</p>
    </div>

    ${blWarning}

    <div class="summary-card">
      <div class="summary-row">
        <span class="summary-label">📀 תיקיית יעד</span>
        <span class="summary-value">${escHtml(state.destFolder)}</span>
      </div>
      <div class="summary-row">
        <span class="summary-label">💾 קובץ Brunch</span>
        <span class="summary-value">${escHtml(state.imgFileName)}</span>
      </div>
      <div class="summary-row">
        <span class="summary-label">📦 גודל קובץ</span>
        <span class="summary-value">${formatBytes(state.imgSize)}</span>
      </div>
      <div class="summary-row">
        <span class="summary-label">⏱ זמן משוער</span>
        <span class="summary-value">~${estMin} דקות</span>
      </div>
    </div>

    <div class="actions-list">
      <div class="action-item success">
        <span class="action-icon">✓</span>
        <span>Fast Startup יושבת (Registry: HiberbootEnabled=0)</span>
      </div>
      <div class="action-item success">
        <span class="action-icon">✓</span>
        <span>ערך GRUB2 ייכתב ל: ${escHtml(state.destFolder)}\\grub_entry.txt</span>
      </div>
      <div class="action-item success">
        <span class="action-icon">✓</span>
        <span>העתקה עם Buffered Streams (8MB chunks) — ללא קריסת UI</span>
      </div>
    </div>

    <div class="step-actions">
      <button class="btn btn-ghost" onclick="window._prevStep()">
        &rarr; חזור
      </button>
      <button class="btn btn-primary btn-install"
              onclick="window._startInstall()">
        🚀 &nbsp;התחל התקנה
      </button>
    </div>
  `);
}

// ─────────────────────────────────────────────
// שלב 4: מסך התקנה + Progress Bar
// ─────────────────────────────────────────────

function renderStep4(): void {
  html('step-content', `
    <div class="step-header">
      <h2>מתקין...</h2>
      <p class="subtitle" id="install-status">מאתחל...</p>
    </div>

    <div class="progress-container">
      <div class="progress-bar-outer">
        <div class="progress-bar-inner" id="prog-bar" style="width:0%"></div>
      </div>
      <div class="progress-info">
        <span id="prog-pct" style="font-weight:600;color:var(--accent)">0%</span>
        <span id="prog-speed"></span>
        <span id="prog-size"></span>
      </div>
    </div>

    <div class="install-log" id="install-log"></div>

    <p class="install-note">
      ⚠️ &nbsp;אל תסגור חלון זה בזמן ההתקנה
    </p>
  `);
}

async function startInstall(): Promise<void> {
  goToStep(4);

  // התחלת האזנה לאירועי progress מ-Rust
  state.unlistenFn = await listen<ProgressPayload>(
    'installation-progress',
    ev => updateProgress(ev.payload)
  );

  try {
    await invoke('start_installation', {
      imgPath:     state.imgPath,
      destFolder:  state.destFolder,
      imgFilename: state.imgFileName,
    });

    // הצלחה — מעבר למסך סיום
    await handleDone();

  } catch (e: unknown) {
    if (state.unlistenFn) await state.unlistenFn();
    showError(`שגיאה בהתקנה:\n${String(e)}`);
  }
}

function updateProgress(data: ProgressPayload): void {
  const bar    = $('prog-bar')   as HTMLElement;
  const pct    = $('prog-pct')   as HTMLElement;
  const speed  = $('prog-speed') as HTMLElement;
  const size   = $('prog-size')  as HTMLElement;
  const status = $('install-status') as HTMLElement;
  const log    = $('install-log') as HTMLElement;

  // עדכון Progress Bar
  bar.style.width = `${Math.min(data.percentage, 100).toFixed(1)}%`;
  pct.textContent = `${data.percentage.toFixed(1)}%`;

  if (data.speed_mbps > 0.1)
    speed.textContent = `${data.speed_mbps.toFixed(1)} MB/s`;

  if (data.total_bytes > 0)
    size.textContent =
      `${formatBytes(data.bytes_copied)} / ${formatBytes(data.total_bytes)}`;

  // עדכון הודעת סטטוס
  if (data.status) status.textContent = data.status;

  // הוספת הודעות ל-log (לא הודעות "מעתיק..." שחוזרות כל הזמן)
  if (data.status && !data.status.startsWith('מעתיק')) {
    const entry = document.createElement('div');
    entry.className = 'log-entry';
    entry.textContent = `• ${data.status}`;
    log.appendChild(entry);
    log.scrollTop = log.scrollHeight;
  }
}

// ─────────────────────────────────────────────
// שלב 5: סיום
// ─────────────────────────────────────────────

async function handleDone(): Promise<void> {
  if (state.unlistenFn) {
    await state.unlistenFn();
    state.unlistenFn = null;
  }

  try {
    state.grubEntry = await invoke<string>('get_grub_entry_content', {
      imgFilename: state.imgFileName,
      destFolder:  state.destFolder,
    });
  } catch {
    state.grubEntry = '(שגיאה בטעינת ערך GRUB)';
  }

  goToStep(5);
}

function renderStep5(): void {
  html('step-content', `
    <div class="step-header success">
      <div class="success-icon">✅</div>
      <h2>ההתקנה הושלמה!</h2>
      <p class="subtitle">
        ChromeOS הועתק בהצלחה. כעת יש להוסיף את ערך ה-GRUB.
      </p>
    </div>

    <div class="done-instructions">
      <h3 style="color:var(--text-primary);margin-bottom:10px;font-size:15px">
        שלבים הבאים:
      </h3>
      <ol>
        <li>הקובץ <code>${escHtml(state.destFolder)}\\${escHtml(state.imgFileName)}</code> נמצא במקומו ✓</li>
        <li>ערך ה-GRUB נשמר ב: <code>${escHtml(state.destFolder)}\\grub_entry.txt</code></li>
        <li>פתח את <code>/boot/grub/grub.cfg</code> מ-Linux והוסף את הטקסט הבא:</li>
      </ol>
    </div>

    <div class="grub-entry-box">
      <div class="grub-entry-header">
        <span>ערך GRUB2 — העתק לתוך grub.cfg</span>
        <button class="btn-copy" onclick="window._copyGrub()">📋 העתק</button>
      </div>
      <pre class="grub-code" id="grub-code-pre">${escHtml(state.grubEntry)}</pre>
    </div>

    <div class="step-actions">
      <div></div>
      <button class="btn btn-ghost" onclick="window.location.reload()">
        🔄 &nbsp;התקנה חדשה
      </button>
    </div>
  `);
}

async function copyGrub(): Promise<void> {
  try {
    await navigator.clipboard.writeText(state.grubEntry);
    const btn = document.querySelector<HTMLElement>('.btn-copy');
    if (btn) {
      btn.textContent = '✓ הועתק!';
      setTimeout(() => { btn.textContent = '📋 העתק'; }, 2000);
    }
  } catch {
    // אם clipboard API לא זמין
    alert('לא ניתן להעתיק — העתק ידנית מהתיבה');
  }
}

// ─────────────────────────────────────────────
// ניהול ניווט בין שלבים
// ─────────────────────────────────────────────

function goToStep(step: number): void {
  state.step = step;
  showScreen('screen-main');
  updateDots();

  switch (step) {
    case 1: renderStep1(); break;
    case 2: renderStep2(); break;
    case 3: renderStep3(); break;
    case 4: renderStep4(); break;
    case 5: renderStep5(); break;
  }
}

function updateDots(): void {
  for (let i = 1; i <= 5; i++) {
    const dot = $(`step-dot-${i}`);
    if (!dot) continue;
    dot.className = 'step-dot';
    if (i < state.step)  dot.classList.add('done');
    if (i === state.step) dot.classList.add('active');
  }
}

function showError(msg: string): void {
  html('error-msg', escHtml(msg));
  showScreen('screen-error');
}

// ─────────────────────────────────────────────
// חשיפת פונקציות ל-window (עבור onclick ב-HTML)
// TypeScript strict mode מחייב גישה דרך window
// ─────────────────────────────────────────────

type GlobalFn = (...args: unknown[]) => unknown;
const W = window as unknown as Record<string, GlobalFn>;

W._driveSelected = (letter: unknown) => driveSelected(letter as string);
W._selectFile    = () => selectFile();
W._nextStep      = () => goToStep(state.step + 1);
W._prevStep      = () => goToStep(state.step - 1);
W._startInstall  = () => startInstall();
W._copyGrub      = () => copyGrub();

// ─────────────────────────────────────────────
// הפעלה
// ─────────────────────────────────────────────

document.addEventListener('DOMContentLoaded', init);
