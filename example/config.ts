import { google } from 'googleapis'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import dotenv from 'dotenv'
import process from "node:process"
import credentials from './credentials.json' with { type: 'json' }

const __dirname = dirname(fileURLToPath(import.meta.url))

dotenv.config({ path: resolve(__dirname, './.env') })

export const SHEET_ID = process.env.SHEET_ID as string

export const languages = ['en', 'zh', 'ja', 'fr', 'es'] as const

export const auth = new google.auth.GoogleAuth({
  credentials,
  scopes: 'https://www.googleapis.com/auth/spreadsheets',
})

export const folderPath = resolve(__dirname + '/i18n')
