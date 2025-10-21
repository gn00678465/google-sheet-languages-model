import { existsSync } from 'node:fs'
import { resolve, extname } from 'node:path'
import { pathToFileURL } from 'node:url'
import type { LanguagesContentType } from '../types.ts'

export interface CLIConfig {
  sheetId?: string
  sheetTitle?: string
  credentials?: string | Record<string, unknown>
  languages?: string[]
  type?: LanguagesContentType
  directory?: string
}

/**
 * Load configuration from a file (supports .js, .ts, .mjs, .cjs)
 */
export async function loadConfig(
  configPath: string
): Promise<CLIConfig> {
  const absolutePath = resolve(configPath)

  if (!existsSync(absolutePath)) {
    throw new Error(`Config file not found: ${absolutePath}`)
  }

  const ext = extname(absolutePath)
  const supportedExtensions = ['.js', '.ts', '.mjs', '.cjs']

  if (!supportedExtensions.includes(ext)) {
    throw new Error(
      `Unsupported config file extension: ${ext}. Supported: ${supportedExtensions.join(', ')}`
    )
  }

  try {
    // Use dynamic import with file URL for better cross-platform support
    const fileUrl = pathToFileURL(absolutePath).href
    const module = await import(fileUrl)

    // Support both default export and named exports
    const config: CLIConfig = module.default || module

    // Validate that we have a valid configuration object
    if (!config || (typeof config === 'object' && Object.keys(config).length === 0)) {
      throw new Error(
        `Config file ${absolutePath} has no valid exports. Please ensure it exports a configuration object (default export or named exports).`
      )
    }

    return config
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error)
    
    // Provide more specific error context based on error type
    if (errorMessage.includes('SyntaxError')) {
      throw new Error(
        `Failed to load config file due to syntax error in ${absolutePath}:\n${errorMessage}`
      )
    } else if (errorMessage.includes('Cannot find module')) {
      throw new Error(
        `Failed to load config file - missing dependency in ${absolutePath}:\n${errorMessage}`
      )
    } else {
      throw new Error(
        `Failed to load config file ${absolutePath}:\n${errorMessage}\n\nPlease ensure the file is a valid JavaScript/TypeScript module with proper exports.`
      )
    }
  }
}

/**
 * Merge CLI arguments with config file, CLI arguments take precedence
 */
export function mergeConfig(
  config: CLIConfig,
  args: Partial<CLIConfig>
): CLIConfig {
  return {
    sheetId: args.sheetId ?? config.sheetId,
    sheetTitle: args.sheetTitle ?? config.sheetTitle,
    credentials: args.credentials ?? config.credentials,
    languages: args.languages ?? config.languages,
    type: args.type ?? config.type,
    directory: args.directory ?? config.directory,
  }
}

/**
 * Extract explicitly provided CLI arguments (ignore yargs defaults)
 * Only returns arguments that were actually provided by the user
 */
export function extractExplicitArgs(
  argv: Record<string, unknown>,
  keys: string[]
): Record<string, unknown> {
  const explicitArgs: Record<string, unknown> = {}
  
  for (const key of keys) {
    if (key in argv) {
      explicitArgs[key] = argv[key]
    }
  }
  
  return explicitArgs
}

/**
 * Load config file and merge with CLI arguments
 * Returns merged configuration with CLI args taking precedence
 */
export async function loadAndMergeConfig<T extends CLIConfig>(
  configPath: string,
  argv: Record<string, unknown>,
  argKeys: string[]
): Promise<T> {
  const configData = await loadConfig(configPath)
  const explicitArgs = extractExplicitArgs(argv, argKeys)
  return mergeConfig(configData, explicitArgs) as T
}
