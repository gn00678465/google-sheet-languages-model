import { google } from 'googleapis'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

export interface AuthOptions {
  credentialsPath?: string
  credentials?: object
  useEnv?: boolean
}

/**
 * Create Google Auth instance with various authentication methods
 * Priority: credentials object > credentialsPath > environment variable
 */
export function createAuth(options: AuthOptions = {}) {
  const { credentialsPath, credentials, useEnv = true } = options

  const scopes = ['https://www.googleapis.com/auth/spreadsheets']

  // 1. Use credentials object directly
  if (credentials) {
    return new google.auth.GoogleAuth({
      credentials,
      scopes,
    })
  }

  // 2. Use credentials file path
  if (credentialsPath) {
    const resolvedPath = resolve(credentialsPath)
    const credentialsData = JSON.parse(readFileSync(resolvedPath, 'utf8'))
    return new google.auth.GoogleAuth({
      credentials: credentialsData,
      scopes,
    })
  }

  // 3. Use environment variable GOOGLE_APPLICATION_CREDENTIALS
  if (useEnv) {
    return new google.auth.GoogleAuth({
      scopes,
    })
  }

  throw new Error(
    'No credentials provided. Please provide credentials via --credentials flag or GOOGLE_APPLICATION_CREDENTIALS environment variable.'
  )
}

/**
 * Validate credentials file format
 */
export function validateCredentials(credentials: unknown): boolean {
  if (!credentials || typeof credentials !== 'object') {
    return false
  }
  
  const creds = credentials as Record<string, unknown>

  // Check for service account credentials
  if (creds.type === 'service_account') {
    return !!(
      creds.project_id &&
      creds.private_key &&
      creds.client_email
    )
  }

  return false
}
