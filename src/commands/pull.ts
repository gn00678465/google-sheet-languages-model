import type { CommandModule } from 'yargs'
import { resolve } from 'node:path'
import { mkdirSync } from 'node:fs'
import process from 'node:process'
import { GoogleSheetLanguagesModel } from '../core/GoogleSheetLanguagesModel.ts'
import type { Languages } from '../core/LanguagesModel.ts'
import type { LanguagesContentType } from '../types.ts'
import { createAuth } from '../utils/auth.ts'
import { logger } from '../utils/logger.ts'
import {
  validateSheetId,
  validateDirectory,
  validateFile,
} from '../utils/validator.ts'
import { loadConfig, mergeConfig } from '../utils/config.ts'

interface PullCommandArgs {
  config?: string
  sheetId?: string
  sheetTitle?: string
  credentials?: string | Record<string, unknown>
  directory?: string
  languages?: string[]
  type?: LanguagesContentType
}

export const command: CommandModule = {
  command: 'pull',
  describe: 'Pull i18n translations from Google Sheet to local files',

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
        describe: 'Directory for translation files',
        alias: 'd',
      })
      .option('languages', {
        type: 'array',
        describe: 'Language codes (e.g., en zh ja fr es)',
        alias: 'l',
      })
      .option('type', {
        type: 'string',
        choices: ['nest', 'flat'] as const,
        describe: 'Output file structure type',
      })
      .example([
        [
          '$0 pull --config="./gslm.config.js"',
          'Pull using config file',
        ],
        [
          '$0 pull --sheet-id="abc123" --sheet-title="i18n" --credentials="./credentials.json" --languages en zh ja',
          'Pull translations from Google Sheet',
        ],
        [
          '$0 pull -s "abc123" -t "i18n" -c "./creds.json" -l en zh -d "./locales" --type=flat',
          'Pull with short options and flat structure',
        ],
      ])
  },

  async handler(argv: Record<string, unknown>) {
    const startTime = Date.now()
    let args = argv as unknown as PullCommandArgs

    try {
      logger.startCommand('pull')

      // Load and merge config if config file is provided
      if (args.config) {
        logger.info(`Loading config from: ${args.config}`)
        const configData = await loadConfig(args.config)
        
        // Only use CLI args that were explicitly provided (not from yargs defaults)
        const explicitArgs: Partial<PullCommandArgs> = {}
        if ('sheetId' in argv) explicitArgs.sheetId = args.sheetId
        if ('sheetTitle' in argv) explicitArgs.sheetTitle = args.sheetTitle
        if ('credentials' in argv) explicitArgs.credentials = args.credentials
        if ('languages' in argv) explicitArgs.languages = args.languages
        if ('type' in argv) explicitArgs.type = args.type
        if ('directory' in argv) explicitArgs.directory = args.directory
        
        args = mergeConfig(configData, explicitArgs) as PullCommandArgs
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
      
      // Set default type if not provided
      if (!args.type) {
        args.type = 'nest'
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

      // Ensure directory exists
      const directoryPath = resolve(args.directory)
      if (!validateDirectory(args.directory)) {
        logger.info(`Creating directory: ${directoryPath}`)
        mkdirSync(directoryPath, { recursive: true })
      }

      const languages = args.languages

      logger.info(`Sheet ID: ${args.sheetId}`)
      logger.info(`Sheet Title: ${args.sheetTitle}`)
      logger.info(`Languages: ${languages.join(', ')}`)
      logger.info(`Directory: ${directoryPath}`)
      logger.info(`Type: ${args.type}`)

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

      // Load data from Google Sheet
      logger.info('Fetching data from Google Sheet...')
      const languagesModel = await googleSheetModel.loadFromGoogleSheet(
        args.sheetTitle,
        languages as Languages
      )

      // Save to local files
      logger.info('Saving translations to local files...')
      languagesModel.saveToFolder(directoryPath, args.type || 'nest')

      const duration = Date.now() - startTime
      logger.completeCommand('pull', duration)

      logger.log('')
      logger.success(
        `Translations saved to: ${directoryPath}`
      )
      logger.success(
        `Files created: ${languages.map((lang: string) => `${lang}.json`).join(', ')}`
      )
    } catch (error) {
      logger.error(
        error instanceof Error ? error.message : String(error)
      )
      process.exit(1)
    }
  },
}

export default command
