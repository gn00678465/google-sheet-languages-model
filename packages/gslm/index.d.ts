// Hand-written: re-exports the generated binding typings and refines the
// parts that index.js wraps.
export {
  flatten,
  unflatten,
  sheetToModel,
  modelToSheet,
  orphanKeys,
  version,
  type Model,
  type CredentialsOptions,
  type SheetsClientOptions,
} from './binding'

/**
 * Safe credentials metadata. `kind` determines which optional field exists;
 * credential JSON is never returned to JavaScript.
 */
export interface ConfigCredentials {
  kind: 'file' | 'json' | 'adc'
  path?: string
  env?: string
}

/** A fully resolved Target with an absolute local Catalog path template. */
export interface ResolvedTarget {
  name: string
  sheet: string
  tab: string
  locales: string[]
  path: string
  format: 'nest' | 'flat'
  keySeparator: string
  credentials: ConfigCredentials
}

/** Result of `loadConfig`, including any discovery warnings. */
export interface ResolvedConfig {
  configPath?: string
  targets: ResolvedTarget[]
  warnings: string[]
}

/**
 * Explicit values that take precedence over `GSLM_*` and the config file.
 * `credentials` and `credentialsJson` are mutually exclusive.
 */
export interface ConfigOverrides {
  sheet?: string
  tab?: string
  locales?: string[]
  path?: string
  format?: 'nest' | 'flat'
  keySeparator?: string
  credentials?: string
  credentialsJson?: string
}

/** Options for config discovery, loading, and Target selection. */
export interface LoadConfigOptions {
  cwd?: string
  configPath?: string
  env?: Record<string, string>
  overrides?: ConfigOverrides
  targets?: string[]
  loadDotenv?: boolean
}

/** Error shape thrown by `loadConfig`; `code` is stable for programmatic use. */
export interface ConfigError extends Error {
  code:
    | 'CONFIG_NOT_FOUND'
    | 'CONFIG_PARSE'
    | 'CONFIG_INVALID'
    | 'CONFIG_LEGACY'
    | 'CONFIG_UNSUPPORTED'
    | 'CONFIG_UNSUPPORTED_VERSION'
}

/** Discover, validate, and resolve config into safe, absolute Target data. */
export declare function loadConfig(options?: LoadConfigOptions | undefined | null): ResolvedConfig
/** JSON Schema draft 2020-12 generated from the Rust config types. */
export declare function configSchema(): Record<string, unknown>

/** TOML preview and non-fatal migration warnings from a legacy config object. */
export interface LegacyMigrationResult {
  toml: string
  warnings: string[]
}

/** Convert a loaded legacy executable-config object into safe TOML text. */
export declare function migrateLegacyConfig(legacy: unknown): LegacyMigrationResult

/** Stable error codes set on `error.code` by `SheetsClient` methods. */
export type SheetsErrorCode =
  | 'CREDENTIALS'
  | 'AUTH'
  | 'PERMISSION_DENIED'
  | 'SHEET_NOT_FOUND'
  | 'TAB_NOT_FOUND'
  | 'RATE_LIMITED'
  | 'SERVER_ERROR'
  | 'HTTP'
  | 'NETWORK'
  | 'INVALID_RESPONSE'
  | 'WRITE_AFTER_CLEAR_FAILED'

export interface SheetsError extends Error {
  code: SheetsErrorCode
}

/** Reads and writes whole Tabs of a Google Sheet. */
export declare class SheetsClient {
  private constructor()
  /**
   * Create a client. Credentials are validated here (file readable, JSON is a
   * service-account key); no token is exchanged yet. Rejects with a
   * `SheetsError`.
   */
  static create(options?: SheetsClientOptions | undefined | null): Promise<SheetsClient>
  /**
   * Create a client from a Target returned by `loadConfig`. JSON credentials
   * are re-read from the process environment or that call's cwd `.env` file.
   */
  static fromConfig(target: ResolvedTarget): Promise<SheetsClient>
  /** Read the whole tab as rows of strings (header row first). Feed the result to `sheetToModel`. */
  readTab(sheetId: string, tab: string): Promise<Array<Array<string>>>
  /** Replace the tab's content with `rows` (clear, then write from A1 with RAW input). */
  writeTab(sheetId: string, tab: string, rows: Array<Array<string>>): Promise<void>
}
