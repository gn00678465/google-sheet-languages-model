// gslm.config.with-credentials-object.js
// Example: Using imported credentials object instead of file path
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import dotenv from 'dotenv'
import process from "node:process"
import credentials from './credentials.json' with { type: 'json' }

const __dirname = dirname(fileURLToPath(import.meta.url))

dotenv.config({ path: resolve(__dirname, './.env') })

export default {
  // Google Sheet ID (get from Sheet URL: https://docs.google.com/spreadsheets/d/[SHEET_ID]/edit)
  sheetId: process.env.SHEET_ID || 'YOUR_SHEET_ID_HERE',
  
  // Sheet title/tab name in your Google Sheet
  sheetTitle: 'i18n-demo',
  
  // Import credentials as object (alternative to file path string)
  credentials: credentials,
  
  // Language codes to sync (ISO 639-1 format)
  languages: ['en', 'zh', 'ja', 'fr', 'es'],
  
  // Directory for translation files (used for both pull and push)
  directory: './example/i18n',
  
  // File structure type: 'nest' (nested object) or 'flat' (dot notation)
  // Note: Only used for pull command. Push automatically detects the structure.
  type: 'flat',
}
