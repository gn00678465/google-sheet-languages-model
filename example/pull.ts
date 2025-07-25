import type { LanguagesContentType } from '../dist/index.d.ts' // Ensure types are loaded
import { GoogleSheetLanguagesModel } from '../dist/index.js'
import { auth, folderPath, languages, SHEET_ID } from './config.ts'
import process from "node:process"

const googleSheetLanguagesModel = new GoogleSheetLanguagesModel({
  sheetId: SHEET_ID,
  auth,
})

const languagesModel = await googleSheetLanguagesModel.loadFromGoogleSheet(
  'i18n-demo',
  languages,
)

const _type = process.argv.find((str) => str.startsWith('--type='))?.replace(
  '--type=',
  '',
) as LanguagesContentType | undefined

const type: LanguagesContentType =
  _type && (['nest', 'flat'] as LanguagesContentType[]).includes(_type)
    ? _type
    : 'nest'

languagesModel.saveToFolder(folderPath, type)

console.log('pull done')
