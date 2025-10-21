import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { mkdirSync, writeFileSync, rmSync } from 'node:fs'
import { join } from 'node:path'
import {
  validateSheetId,
  validatePathExists,
  validateDirectory,
  validateFile,
  validateLanguageCodes,
  validateContentType,
} from '../utils/validator.ts'

describe('validator', () => {
  const testDir = join(__dirname, 'tmp', 'validator-test')
  const testFile = join(testDir, 'test.txt')

  beforeAll(() => {
    // Create test directory and file
    mkdirSync(testDir, { recursive: true })
    writeFileSync(testFile, 'test content')
  })

  afterAll(() => {
    // Clean up test directory
    rmSync(testDir, { recursive: true, force: true })
  })

  describe('validateSheetId', () => {
    it('should return true for valid Google Sheet ID', () => {
      const validIds = [
        '1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms',
        '1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t1u2v',
        'abc123-def456_ghi789',
        '12345678901234567890', // Minimum 20 characters
      ]

      validIds.forEach((id) => {
        expect(validateSheetId(id)).toBe(true)
      })
    })

    it('should return false for invalid Google Sheet ID', () => {
      const invalidIds = [
        '', // Empty string
        'short', // Too short
        '1234567890123456789', // Less than 20 characters
        'invalid@sheet#id', // Invalid characters
        'invalid sheet id with spaces',
        'invalid/sheet\\id',
      ]

      invalidIds.forEach((id) => {
        expect(validateSheetId(id)).toBe(false)
      })
    })
  })

  describe('validatePathExists', () => {
    it('should return true for existing path', () => {
      expect(validatePathExists(testDir)).toBe(true)
      expect(validatePathExists(testFile)).toBe(true)
    })

    it('should return false for non-existing path', () => {
      expect(validatePathExists(join(testDir, 'non-existing'))).toBe(false)
    })

    it('should handle relative paths', () => {
      expect(validatePathExists('.')).toBe(true)
      expect(validatePathExists('./non-existing-path')).toBe(false)
    })
  })

  describe('validateDirectory', () => {
    it('should return true for existing directory', () => {
      expect(validateDirectory(testDir)).toBe(true)
    })

    it('should return false for file path', () => {
      expect(validateDirectory(testFile)).toBe(false)
    })

    it('should return false for non-existing directory', () => {
      expect(validateDirectory(join(testDir, 'non-existing'))).toBe(false)
    })

    it('should handle relative paths', () => {
      expect(validateDirectory('.')).toBe(true)
    })
  })

  describe('validateFile', () => {
    it('should return true for existing file', () => {
      expect(validateFile(testFile)).toBe(true)
    })

    it('should return false for directory path', () => {
      expect(validateFile(testDir)).toBe(false)
    })

    it('should return false for non-existing file', () => {
      expect(validateFile(join(testDir, 'non-existing.txt'))).toBe(false)
    })
  })

  describe('validateLanguageCodes', () => {
    it('should return true for valid language codes', () => {
      const validCodes = [
        ['en', 'es', 'fr'],
        ['zh', 'ja', 'ko'],
        ['en-US', 'zh-CN', 'pt-BR'],
        ['zh-Hans', 'zh-Hant'],
        ['en-Latn-US'],
      ]

      validCodes.forEach((codes) => {
        expect(validateLanguageCodes(codes)).toBe(true)
      })
    })

    it('should return false for invalid language codes', () => {
      const invalidCodes = [
        [], // Empty array
        [''], // Empty string
        ['invalid'], // Too long
        ['EN'], // Uppercase
        ['en US'], // Space
        ['en_US'], // Underscore instead of hyphen
        ['1en'], // Starts with number
      ]

      invalidCodes.forEach((codes) => {
        expect(validateLanguageCodes(codes)).toBe(false)
      })
    })

    it('should return false for non-array input', () => {
      expect(validateLanguageCodes(null as unknown as string[])).toBe(false)
      expect(validateLanguageCodes(undefined as unknown as string[])).toBe(false)
      expect(validateLanguageCodes('en' as unknown as string[])).toBe(false)
    })

    it('should return false if any language code is invalid', () => {
      expect(validateLanguageCodes(['en', 'INVALID', 'fr'])).toBe(false)
      expect(validateLanguageCodes(['en', 'es', ''])).toBe(false)
    })
  })

  describe('validateContentType', () => {
    it('should return true for valid content types', () => {
      expect(validateContentType('nest')).toBe(true)
      expect(validateContentType('flat')).toBe(true)
    })

    it('should return false for invalid content types', () => {
      expect(validateContentType('invalid')).toBe(false)
      expect(validateContentType('NEST')).toBe(false)
      expect(validateContentType('FLAT')).toBe(false)
      expect(validateContentType('')).toBe(false)
      expect(validateContentType(null as unknown as string)).toBe(false)
      expect(validateContentType(undefined as unknown as string)).toBe(false)
    })

    it('should provide type guard functionality', () => {
      const type = 'nest'
      if (validateContentType(type)) {
        // TypeScript should narrow the type here
        const validType: 'nest' | 'flat' = type
        expect(validType).toBe('nest')
      }
    })
  })
})
