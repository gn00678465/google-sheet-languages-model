import { existsSync, statSync } from 'node:fs'
import { resolve } from 'node:path'

/**
 * Validate Google Sheet ID format
 */
export function validateSheetId(sheetId: string): boolean {
  // Google Sheet ID is typically 44 characters long
  // Format: alphanumeric, hyphens, and underscores
  const sheetIdRegex = /^[a-zA-Z0-9-_]{20,}$/
  return sheetIdRegex.test(sheetId)
}

/**
 * Validate if path exists
 */
export function validatePathExists(path: string): boolean {
  const resolvedPath = resolve(path)
  return existsSync(resolvedPath)
}

/**
 * Validate if path is a directory
 */
export function validateDirectory(path: string): boolean {
  const resolvedPath = resolve(path)
  if (!existsSync(resolvedPath)) {
    return false
  }
  return statSync(resolvedPath).isDirectory()
}

/**
 * Validate if path is a file
 */
export function validateFile(path: string): boolean {
  const resolvedPath = resolve(path)
  if (!existsSync(resolvedPath)) {
    return false
  }
  return statSync(resolvedPath).isFile()
}

/**
 * Validate language codes format
 * Language codes should be 2-5 characters (ISO 639-1 or locale format)
 */
export function validateLanguageCodes(languages: string[]): boolean {
  if (!Array.isArray(languages) || languages.length === 0) {
    return false
  }

  const langRegex = /^[a-z]{2,3}(-[A-Z][a-z]{3})?(-[A-Z]{2})?$/
  return languages.every((lang) => langRegex.test(lang))
}

/**
 * Validate content type
 */
export function validateContentType(
  type: string
): type is 'nest' | 'flat' {
  return type === 'nest' || type === 'flat'
}
