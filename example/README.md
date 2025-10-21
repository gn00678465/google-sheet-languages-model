# Example Usage

This directory demonstrates both **CLI** and **Programmatic API** usage of `google-sheet-languages-model`.

## Prerequisites

1. **Google Service Account Credentials**
   - Place your `credentials.json` file in this directory
   - See [Authorization Guide](../README.md#authorization) for setup instructions

2. **Environment Variables**
   - Copy `.env.example` to `.env`
   - Set your `SHEET_ID` in the `.env` file

## Method 1: CLI with Config File (Recommended)

The CLI approach is simpler and requires no code.

### Configuration

Edit `gslm.config.js` to match your settings:

**Option 1: Using credentials file path (string)**

```javascript
export default {
  sheetId: process.env.SHEET_ID || 'YOUR_SHEET_ID_HERE',
  sheetTitle: 'i18n-demo',
  credentials: './credentials.json', // File path
  languages: ['en', 'zh', 'ja', 'fr', 'es'],
  directory: './i18n', // Used for both input and output
  type: 'nest',
}
```

**Option 2: Importing credentials directly (object)**

```javascript
import credentials from './credentials.json' with { type: 'json' }

export default {
  sheetId: process.env.SHEET_ID,
  sheetTitle: 'i18n-demo',
  credentials: credentials, // Import object directly
  languages: ['en', 'zh', 'ja', 'fr', 'es'],
  directory: './i18n',
  type: 'nest',
}
```

**What's the difference?**

- **String path**: CLI reads the file at runtime (more flexible, can change credentials without restarting)
- **Object import**: Credentials are bundled into config (simpler, but requires rebuild if credentials change)

Both methods work identically for authentication!

### Usage

```bash
# Pull translations from Google Sheet
pnpm example:cli:pull

# Push translations to Google Sheet
pnpm example:cli:push
```

**Advantages:**
- ✅ No TypeScript code needed
- ✅ Cross-platform compatible (Windows/Mac/Linux)
- ✅ Clean and maintainable
- ✅ Easy to understand for non-developers

## Method 2: Programmatic API

The programmatic API provides more flexibility for integration into your application.

### Configuration

Edit `config.ts` to set up authentication and paths:

```typescript
import { google } from 'googleapis'

export const SHEET_ID = process.env.SHEET_ID as string
export const languages = ['en', 'zh', 'ja', 'fr', 'es'] as const
export const auth = new google.auth.GoogleAuth({
  credentials,
  scopes: 'https://www.googleapis.com/auth/spreadsheets',
})
export const folderPath = resolve(__dirname + '/i18n')
```

### Usage

```bash
# Pull translations (nested structure)
pnpm example:api:pull:nest

# Pull translations (flat structure)
pnpm example:api:pull:flat

# Push translations
pnpm example:api:push
```

**Advantages:**
- ✅ Full programmatic control
- ✅ Can integrate into build processes
- ✅ Custom logic and transformations
- ✅ Type-safe with TypeScript

## Which Method Should I Use?

### Use CLI if:
- You just need to sync translations between Google Sheets and local files
- You prefer configuration over code
- You want a simple, no-code solution
- You're setting up a workflow for non-developers

### Use Programmatic API if:
- You need to integrate translation sync into your application
- You need custom processing or transformations
- You're building a more complex i18n pipeline
- You need fine-grained control over the sync process

## Files

- `gslm.config.js` - CLI configuration file
- `config.ts` - Programmatic API configuration
- `pull.ts` - Programmatic pull example
- `push.ts` - Programmatic push example
- `credentials.json` - Google Service Account credentials (not in repo)
- `.env` - Environment variables (not in repo)
- `.env.example` - Example environment variables
- `i18n/` - Translation files directory

## Learn More

- [Main Documentation](../README.md)
- [CLI Usage](../README.md#cli-usage)
- [Programmatic API Usage](../README.md#programmatic-api-usage)
