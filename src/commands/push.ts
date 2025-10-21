import type { CommandModule } from 'yargs'
import { resolve } from 'node:path'
import process from 'node:process'
import { GoogleSheetLanguagesModel } from '../core/GoogleSheetLanguagesModel.ts'
import type { Languages } from '../core/LanguagesModel.ts'
import { createAuth } from '../utils/auth.ts'
import { logger } from '../utils/logger.ts'
import {
  validateSheetId,
  validateDirectory,
  validateFile,
} from '../utils/validator.ts'
import { loadAndMergeConfig } from '../utils/config.ts'

interface PushCommandArgs {
  config?: string
  sheetId?: string
  sheetTitle?: string
  credentials?: string | Record<string, unknown>
  directory?: string
  languages?: string[]
}

export const command: CommandModule = {
  command: 'push',
  describe: 'Push local i18n translations to Google Sheet',

  builder(yargs) {
    return yargs
      .option('config', {
        type: 'string',
        describe: 'Path to config file (.js, .ts, .mjs, .cjs)',
        alias: 'C',
      })
      .option('sheet-id', {
        type: 'string',
        describe: 'Google Sheet ID (from the URL)',
        alias: 's',
      })
      .option('sheet-title', {
        type: 'string',
        describe: 'Sheet title/tab name',
        alias: 't',
      })
      .option('credentials', {
        type: 'string',
        describe:
          'Path to credentials.json file (or use GOOGLE_APPLICATION_CREDENTIALS env var)',
        alias: 'c',
      })
      .option('directory', {
        type: 'string',
        describe: 'Directory containing translation files',
        alias: 'd',
      })
      .option('languages', {
        type: 'array',
        describe: 'Language codes (e.g., en zh ja fr es)',
        alias: 'l',
      })
      .example([
        [
          '$0 push --config="./gslm.config.js"',
          'Push using config file',
        ],
        [
          '$0 push --sheet-id="abc123" --sheet-title="i18n" --credentials="./credentials.json" --languages en zh ja',
          'Push translations to Google Sheet',
        ],
        [
          '$0 push -s "abc123" -t "i18n" -c "./creds.json" -l en zh -d "./locales"',
          'Push with short options',
        ],
      ])
  },

  async handler(argv: Record<string, unknown>) {
    const startTime = Date.now()
    let args = argv as unknown as PushCommandArgs

    try {
      logger.startCommand('push')

      // Load and merge config if config file is provided
      if (args.config) {
        logger.info(`Loading config from: ${args.config}`)
        args = await loadAndMergeConfig<PushCommandArgs>(
          args.config,
          argv,
          ['sheetId', 'sheetTitle', 'credentials', 'languages', 'directory']
        )
        logger.success('Config loaded successfully')
      }

      // Validate required fields
      if (!args.sheetId) {
        throw new Error('sheet-id is required (via CLI argument or config file)')
      }
      if (!args.sheetTitle) {
        throw new Error('sheet-title is required (via CLI argument or config file)')
      }
      if (!args.languages || args.languages.length === 0) {
        throw new Error('languages are required (via CLI argument or config file)')
      }
      
      // Set default directory if not provided
      if (!args.directory) {
        args.directory = './i18n'
      }

      // Validate Sheet ID
      if (!validateSheetId(args.sheetId)) {
        throw new Error(
          `Invalid Sheet ID format: ${args.sheetId}. Please check your Google Sheet URL.`
        )
      }

      // Validate credentials file if provided as string path
      if (args.credentials) {
        if (typeof args.credentials === 'string' && !validateFile(args.credentials)) {
          throw new Error(
            `Credentials file not found: ${args.credentials}`
                )
        }
      }

      // Validate directory
      const directoryPath = resolve(args.directory)
      if (!validateDirectory(args.directory)) {
        throw new Error(
          `Directory not found: ${directoryPath}`
        )
      }

      const languages = args.languages

      logger.info(`Sheet ID: ${args.sheetId}`)
      logger.info(`Sheet Title: ${args.sheetTitle}`)
      logger.info(`Languages: ${languages.join(', ')}`)
      logger.info(`Directory: ${directoryPath}`)

      // Load from local folder
      logger.info('Loading translations from local files...')
      const languagesModel = GoogleSheetLanguagesModel.loadFromFolder(
        directoryPath,
        languages as Languages
      )

      // Create authentication
      logger.info('Authenticating with Google Sheets API...')
      const auth = createAuth({
        credentialsPath: typeof args.credentials === 'string' ? args.credentials : undefined,
        credentials: typeof args.credentials === 'object' ? args.credentials : undefined,
      })

      // Create Google Sheet model instance
      const googleSheetModel = new GoogleSheetLanguagesModel({
        sheetId: args.sheetId,
        auth,
      })

      // Save to Google Sheet
      logger.info('Uploading translations to Google Sheet...')
      await googleSheetModel.saveToGoogleSheet(
        args.sheetTitle,
        languagesModel
      )

      const duration = Date.now() - startTime
      logger.completeCommand('push', duration)

      logger.log('')
      logger.success(
        `Translations uploaded to Google Sheet: ${args.sheetId}`
      )
      logger.success(`Sheet title: ${args.sheetTitle}`)
    } catch (error) {
      logger.error(
        error instanceof Error ? error.message : String(error)
      )
      process.exit(1)
    }
  },
}

export default command
