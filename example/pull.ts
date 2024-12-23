import {
  GoogleSheetLanguagesModel,
  type LanguagesContentType,
} from '../src/index.ts'
import { auth, folderPath, languages, SHEET_ID } from './config.ts'

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
