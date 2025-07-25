import { GoogleSheetLanguagesModel } from '../dist/index.js'
import { auth, folderPath, languages, SHEET_ID } from './config.ts'

const languagesModel = GoogleSheetLanguagesModel.loadFromFolder(
  folderPath,
  languages,
)

const googleSheetLanguagesModel = new GoogleSheetLanguagesModel({
  sheetId: SHEET_ID,
  auth,
})

await googleSheetLanguagesModel.saveToGoogleSheet('i18n-demo', languagesModel)

console.log('push done')
