# Google Sheet Languages Model - i18n google sheet layer

`google-sheet-languages-model` is a npm package that allows you to
download/upload and parse internationalization (i18n) data between a Google
Sheet and your local environment. This package provides both a **CLI tool** and a **programmatic API** for managing your i18n data.

## Quick Start

```bash
# 1. Install the package
pnpm add -D google-sheet-languages-model

# 2. Pull i18n data from Google Sheet
npx gslm pull \
  --sheet-id YOUR_SHEET_ID \
  --sheet-title "Sheet1" \
  --credentials ./credentials.json \
  --directory ./i18n \
  --languages en es fr

# 3. Push i18n data to Google Sheet
npx gslm push \
  --sheet-id YOUR_SHEET_ID \
  --sheet-title "Sheet1" \
  --credentials ./credentials.json \
  --directory ./i18n \
  --languages en es fr
```

## Installation

### For CLI Usage (Recommended)

Install the package globally or as a dev dependency:

```bash
# Global installation
npm install -g google-sheet-languages-model

# Or as dev dependency
pnpm add -D google-sheet-languages-model
```

### For Programmatic API Usage

If you need to use the package programmatically in your code:

```bash
pnpm add -D google-sheet-languages-model
```

> **Note**: Starting from v0.5.0, `googleapis` is now included as a direct dependency. You no longer need to install it separately.

## Authorization

It's recommended to use the
[Service Account](https://developers.google.com/workspace/guides/create-credentials#service-account)
auth option to interact with the Google Sheets API. Follow these steps to set up
the authorization:

1. Enable the Google Sheets API permission for your project in the
   [Google Cloud Console](https://console.cloud.google.com/).
2. Create a new Service Account or use an existing one, and add it as an editor
   to your working sheet.
3. Download the `credentials.json` file which is generated from the Service
   Account and place it in your project directory.
4. Share your Google Sheet with the email address provided in the `client_email`
   field inside the `credentials.json` file.
5. Set the Google Sheet ID from your Google Sheet URL (the part between `/d/` and `/edit`).

## CLI Usage

The CLI tool provides a simple way to sync i18n data without writing any code.

### Pull Command

Download i18n data from Google Sheet to local JSON files:

```bash
# Using npx (if installed locally)
npx gslm pull \
  --sheet-id YOUR_SHEET_ID \
  --sheet-title "Sheet1" \
  --credentials ./credentials.json \
  --directory ./i18n \
  --languages en,es,fr \
  --type nest

# Or using global installation
gslm pull -s YOUR_SHEET_ID -t "Sheet1" -c ./credentials.json -d ./i18n -l en,es,fr
```

**Options:**

- `-s, --sheet-id` (required): Google Sheet ID
- `-t, --sheet-title` (required): Sheet title/tab name
- `-c, --credentials` (required): Path to credentials.json file
- `-d, --directory` (required): Directory for translation files
- `-l, --languages` (required): Comma-separated language codes (e.g., en,es,fr,ja,zh)
- `--type` (optional): Output structure type - `nest` or `flat` (default: nest)

### Push Command

Upload i18n data from local JSON files to Google Sheet:

```bash
# Using npx (if installed locally)
npx gslm push \
  --sheet-id YOUR_SHEET_ID \
  --sheet-title "Sheet1" \
  --credentials ./credentials.json \
  --directory ./i18n \
  --languages en,es,fr

# Or using global installation
gslm push -s YOUR_SHEET_ID -t "Sheet1" -c ./credentials.json -d ./i18n -l en,es,fr
```

**Options:**

- `-s, --sheet-id` (required): Google Sheet ID
- `-t, --sheet-title` (required): Sheet title/tab name
- `-c, --credentials` (required): Path to credentials.json file
- `-d, --directory` (required): Directory containing JSON files
- `-l, --languages` (required): Comma-separated language codes (e.g., en,es,fr,ja,zh)

> **Note**: Push command automatically detects the file structure (nest or flat) from your JSON files.

### Environment Variable

You can also set credentials using the `GOOGLE_APPLICATION_CREDENTIALS` environment variable:

```bash
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/credentials.json
gslm pull -s YOUR_SHEET_ID -t "Sheet1" -d ./i18n -l en,es,fr
```

### Using CLI in npm Scripts

When you install the package locally (non-globally), you can add commands to your `package.json` scripts.

#### Method 1: Using Config File (Recommended)

Create a `gslm.config.js` file in your project root:

```javascript
// gslm.config.js
export default {
  sheetId: process.env.SHEET_ID || 'YOUR_SHEET_ID_HERE',
  sheetTitle: 'i18n-demo',
  credentials: './credentials.json',
  languages: ['en', 'zh', 'ja', 'fr', 'es'],
  directory: './i18n', // Used for both input and output
  type: 'nest', // or 'flat'
}
```

**Alternative: Import credentials directly as object**

```javascript
// gslm.config.js
import credentials from './credentials.json' with { type: 'json' }

export default {
  sheetId: process.env.SHEET_ID,
  sheetTitle: 'i18n-demo',
  credentials: credentials, // Pass the object directly (no file path needed)
  languages: ['en', 'zh', 'ja', 'fr', 'es'],
  directory: './i18n',
  type: 'nest',
}
```

Then in your `package.json`:

```json
{
  "scripts": {
    "i18n:pull": "gslm pull --config gslm.config.js",
    "i18n:push": "gslm push --config gslm.config.js"
  }
}
```

**Benefits:**
- ✅ No cross-platform issues (works on Windows, Mac, Linux)
- ✅ Clean and simple scripts
- ✅ Centralized configuration
- ✅ Can override config values with CLI arguments
- ✅ Can import credentials as object or use file path

**Config Options:**
- `sheetId` (string): Google Sheet ID
- `sheetTitle` (string): Sheet title/tab name
- `credentials` (string | object): Path to credentials.json OR credentials object
- `languages` (string[]): Array of language codes
- `directory` (string): Directory for translation files (used for both pull/push)
- `type` (string): File structure type - 'nest' or 'flat' (only for pull command)

**Supported config file formats:**
- `.js` - JavaScript (ESM)
- `.mjs` - JavaScript (ESM)
- `.cjs` - JavaScript (CommonJS)
- `.ts` - TypeScript (requires tsx or ts-node)

#### Method 2: Direct CLI Arguments

```json
{
  "scripts": {
    "i18n:pull": "gslm pull -s YOUR_SHEET_ID -t Sheet1 -c ./credentials.json -d ./i18n -l en,es,fr,ja,zh",
    "i18n:push": "gslm push -s YOUR_SHEET_ID -t Sheet1 -c ./credentials.json -d ./i18n -l en,es,fr,ja,zh",
    "i18n:pull:flat": "gslm pull -s YOUR_SHEET_ID -t Sheet1 -c ./credentials.json -d ./i18n -l en,es,fr --type flat"
  }
}
```

Then run:

```bash
pnpm i18n:pull   # Pull from Google Sheet
pnpm i18n:push   # Push to Google Sheet
```

#### Method 3: Using Environment Variables

**Using Environment Variables in Scripts:**

You can use `.env` file with environment variables:

```bash
# .env
SHEET_ID=your_google_sheet_id_here
```

Then in your `package.json`:

```json
{
  "scripts": {
    "i18n:pull": "gslm pull -s $SHEET_ID -t Sheet1 -c ./credentials.json -d ./i18n -l en,es,fr",
    "i18n:push": "gslm push -s $SHEET_ID -t Sheet1 -c ./credentials.json -d ./i18n -l en,es,fr"
  }
}
```

> **Note**: On Windows PowerShell, use `$env:SHEET_ID` instead of `$SHEET_ID`, or use a package like `cross-env` for cross-platform compatibility.

#### Mixing Config File with CLI Arguments

CLI arguments take precedence over config file settings:

```bash
# Use config file but override directory
gslm pull --config gslm.config.js --directory ./locales
```

This is useful for:
- Testing different configurations
- Using same config with different directory paths
- Overriding specific values without modifying the config file

## Programmatic API Usage

## Programmatic API Usage

If you need more control or want to integrate the package into your application, you can use the programmatic API.

Firstly, import the necessary modules from the package and your configuration:

```typescript
import { GoogleSheetLanguagesModel } from 'google-sheet-languages-model'
import { auth, folderPath, languages, SHEET_ID } from './config.ts'
```

### Download and Parse Data to Local

Here is an example of how to download and parse i18n data from a Google Sheet
and save it to a local folder:

```typescript
const googleSheetLanguagesModel = new GoogleSheetLanguagesModel({
  sheetId: SHEET_ID,
  auth,
})

const languagesModel = await googleSheetLanguagesModel.loadFromGoogleSheet(
  'Test',
  languages,
)

languagesModel.saveToFolder(folderPath, 'nest')

console.log('pull done')
```

### Upload Data to Google Sheet

Here is an example of how to upload i18n data from a local folder to a Google
Sheet:

```typescript
const languagesModel = GoogleSheetLanguagesModel.loadFromFolder(
  folderPath,
  languages,
)

const googleSheetLanguagesModel = new GoogleSheetLanguagesModel({
  sheetId: SHEET_ID,
  auth,
})

await googleSheetLanguagesModel.saveToGoogleSheet('Test', languagesModel)

console.log('push done')
```

### Configuration

Your `config.ts` should export the following variables:

- `SHEET_ID`: The ID of your Google Sheet.
- `languages`: An array of language codes (e.g., `['en', 'es', 'fr']`).
- `auth`: Your authorization credentials.
- `folderPath`: The path to the folder where you want to save or load the
  language files.

### Examples

For programmatic usage examples:

- Pull i18n from google sheet to local folder.
  ([link](https://github.com/gn00678465/google-sheet-languages-model/blob/main/example/pull.ts))
- Push i18n from local folder to google sheet.
  ([link](https://github.com/gn00678465/google-sheet-languages-model/blob/main/example/push.ts))

## Migration Guide

### Migrating from v0.4.x to v0.5.0

**Breaking Changes:**

1. **`googleapis` is now a direct dependency**: You no longer need to install `googleapis` separately. Remove it from your `package.json` if you were only using it for this package.

   ```bash
   # Before (v0.4.x)
   pnpm add -D google-sheet-languages-model googleapis
   
   # After (v0.5.0)
   pnpm add -D google-sheet-languages-model
   ```

2. **CLI tool added**: The package now includes a CLI tool (`gslm`) that can be used without writing any code. You can still use the programmatic API as before.

**What stays the same:**

- All programmatic APIs remain unchanged and backward compatible
- The same authentication methods are supported
- Configuration files and examples continue to work as before

## Documentation

The main classes and methods are documented in the source code provided. This
will guide you on the data structures and the methods available for use.

Feel free to explore the provided code to better understand how to work with the
`google-sheet-languages-model` package to manage your i18n data.

## Data Structures for i18n File Data

The `google-sheet-languages-model` package supports two different structures to
describe i18n file data: `nest` (JS object style) and `flat` (key annotation
style). Both styles serve to organize your internationalization data in a manner
that best suits your project's needs.

### 1. Nest (JS Object Style)

In the `nest` structure, i18n data is organized in a nested JavaScript object
format, where each key represents a nesting level. This structure is intuitive
and easy to navigate, making it a good choice for projects with a hierarchical
data organization.

Example:

```javascript
{
  "user": {
    "name": "Name",
    "age": "Age"
  },
  "messages": {
    "welcome": "Welcome"
  }
}
```

### 2. Flat (Key Annotation Style)

The `flat` structure, on the other hand, uses a single-level object with keys
representing the path to the value in a dot notation. This structure is simple
and often preferred in projects with a flat data organization.

Example:

```javascript
{
  "user.name": "Name",
  "user.age": "Age",
  "messages.welcome": "Welcome"
}
```

You can choose either of these structures based on your project requirements.
The method `languagesModel.saveToFolder(folderPath, structureStyle)` allows you
to specify the structure style as `'nest'` or `'flat'` when saving the i18n data
to a local folder. Similarly, when loading data from a folder using
`GoogleSheetLanguagesModel.loadFromFolder(folderPath, languages, structureStyle)`,
you can specify the structure style to match the organization of your i18n data.

Example:

```typescript
// Saving data in nest structure
languagesModel.saveToFolder(folderPath, 'nest')

// Or, saving data in flat structure
languagesModel.saveToFolder(folderPath, 'flat')
```

This flexibility allows you to work with the i18n data in a way that's most
convenient and logical for your project's organization.

## Contributing

If you encounter any issues or have features requests, feel free to open an
issue or submit a pull request. Your contributions are welcome!
