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
  /** Read the whole tab as rows of strings (header row first). Feed the result to `sheetToModel`. */
  readTab(sheetId: string, tab: string): Promise<Array<Array<string>>>
  /** Replace the tab's content with `rows` (clear, then write from A1 with RAW input). */
  writeTab(sheetId: string, tab: string, rows: Array<Array<string>>): Promise<void>
}
