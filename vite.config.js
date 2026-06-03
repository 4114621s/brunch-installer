import { defineConfig } from "vite";

// https://vitejs.dev/config/
export default defineConfig({
  // מניעת ניקוי מסך הטרמינל בזמן dev (שימושי לדיבאג)
  clearScreen: false,

  server: {
    port: 5173,
    strictPort: true,  // נכשל אם הפורט תפוס (לא מחפש פורט אחר)
    watch: {
      // לא נעקוב אחרי קבצי Rust — Tauri עושה זאת בעצמו
      ignored: ["**/src-tauri/**"],
    },
  },

  // חשיפת משתני סביבה של Tauri ל-JavaScript
  envPrefix: ["VITE_", "TAURI_ENV_"],

  build: {
    // מטרת הדפדפן: WebView2 של Windows תומך ב-Chrome 105+
    target:
      process.env.TAURI_ENV_PLATFORM === "windows"
        ? "chrome105"
        : "safari13",

    // כיבוי minification בזמן debug (לדיבאג קל יותר)
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,

    // source maps רק ב-debug mode
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
});
