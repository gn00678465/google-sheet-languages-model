// gslm.config.js - Configuration file for Google Sheet Languages Model CLI
// This file allows you to define common settings to avoid repeating CLI arguments

import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import dotenv from 'dotenv'
import process from "node:process"

const __dirname = dirname(fileURLToPath(import.meta.url))

dotenv.config({ path: resolve(__dirname, './.env') })

// Option 1: Use credentials file path (string)
export default {
  // Google Sheet ID (get from Sheet URL: https://docs.google.com/spreadsheets/d/[SHEET_ID]/edit)
  sheetId: process.env.SHEET_ID || 'YOUR_SHEET_ID_HERE',
  
  // Sheet title/tab name in your Google Sheet
  sheetTitle: 'i18n-demo',
  
  // Path to Google Service Account credentials JSON file
  credentials: './example/credentials.json',
  
  // Language codes to sync (ISO 639-1 format)
  languages: ['en', 'zh', 'ja', 'fr', 'es'],
  
  // Directory for translation files (used for both pull and push)
  directory: './example/i18n',
  
  // File structure type: 'nest' (nested object) or 'flat' (dot notation)
  // Note: Only used for pull command. Push automatically detects the structure.
  type: 'nest',
}

// Option 2: Import credentials directly (object)
// Uncomment the following to use imported credentials:
/*
import credentials from './credentials.json' with { type: 'json' }

export default {
  sheetId: process.env.SHEET_ID || 'YOUR_SHEET_ID_HERE',
  sheetTitle: 'i18n-demo',
  credentials: credentials, // Pass the object directly
  languages: ['en', 'zh', 'ja', 'fr', 'es'],
  directory: './i18n',
  type: 'nest',
}
*/
