# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`google-sheet-languages-model` is an npm package that synchronizes internationalization (i18n) data between Google Sheets and local JSON files. It provides both a CLI tool (`gslm`) and a programmatic API for managing multilingual content.

**Repository**: https://github.com/gn00678465/google-sheet-languages-model

## Commands

### Building and Development

```bash
# Build the project (generates ESM and CJS outputs)
pnpm build

# Development mode with watch
pnpm dev

# Type checking
pnpm typecheck
```

### Testing

```bash
# Run tests once
pnpm test

# Watch mode for tests
pnpm test:watch
```

### CLI Examples

```bash
# Pull i18n data from Google Sheet (using config file)
pnpm example:cli:pull

# Push i18n data to Google Sheet (using config file)
pnpm example:cli:push

# Alternative config examples
pnpm example:cli2:pull
pnpm example:cli2:push
```

### Programmatic API Examples

```bash
# Pull example (both nest and flat formats)
pnpm example:api:pull:nest
pnpm example:api:pull:flat

# Push example
pnpm example:api:push
```

## Architecture

### Build System

- **Build tool**: `tsdown` - handles dual format output (ESM + CJS)
- **Entry points**:
  - `src/index.ts` - Main library API (exported as both ESM and CJS)
  - `src/cli.ts` - CLI tool (exported as `gslm` binary)
- **Output**: `dist/` directory with:
  - `index.js` (ESM), `index.cjs` (CommonJS), `index.d.ts` (types)
  - `cli.js` (CLI entry point with shebang)
- **External dependencies**: `googleapis` and `yargs` are marked external
- **Bundled**: `lodash-es` is bundled into the output

### Core Architecture

The project follows a clean separation between data models and Google Sheets integration:

**Core Classes** (in `src/core/`):

1. **`LanguagesModel`** - In-memory representation of i18n data
   - Manages multilingual content in memory
   - Supports two structure types: `nest` (nested object) and `flat` (dot notation)
   - Handles file I/O operations (load from/save to folder)
   - Static factory method: `loadFromFolder(folderPath, languages)`

2. **`GoogleSheetLanguagesModel`** - Google Sheets integration layer
   - Connects to Google Sheets API using service account credentials
   - Bidirectional sync: `loadFromGoogleSheet()` and `saveToGoogleSheet()`
   - Depends on `googleapis` package for authentication and API calls
   - Configuration requires: `sheetId` and Google auth credentials

**Data Flow**:
```
Google Sheet ←→ GoogleSheetLanguagesModel ←→ LanguagesModel ←→ Local JSON Files
```

### CLI Implementation

The CLI (`src/cli.ts`) uses `yargs` for command parsing and supports:

- **Commands**: `pull` (download) and `push` (upload)
- **Configuration**: Supports config files (`.js`, `.mjs`, `.cjs`, `.ts` formats)
- **Credentials**: Can accept either file path (string) or credentials object
- **Language specification**: Array format internally, comma-separated in CLI args

### Configuration Files

Config files (e.g., `example/gslm.config.js`) can export:
- `sheetId`: Google Sheet ID
- `sheetTitle`: Sheet tab name
- `credentials`: Path to credentials.json OR credentials object
- `languages`: Array of language codes (e.g., `['en', 'zh', 'ja', 'fr', 'es']`)
- `directory`: Path for i18n JSON files
- `type`: Structure type - `'nest'` or `'flat'` (only for pull command)

**Note**: Push command auto-detects the structure type from existing JSON files.

## Important Implementation Notes

### Authentication

- Uses Google Service Account credentials (not OAuth)
- Requires Google Sheets API enabled in Google Cloud Console
- Service account email must have editor access to the target sheet
- Credentials can be provided as:
  1. File path string: `'./credentials.json'`
  2. Object: imported credentials JSON
  3. Environment variable: `GOOGLE_APPLICATION_CREDENTIALS`

### Data Structures

**Nest Format** (nested JavaScript object):
```json
{
  "user": {
    "name": "Name",
    "age": "Age"
  }
}
```

**Flat Format** (dot notation):
```json
{
  "user.name": "Name",
  "user.age": "Age"
}
```

### CLI Arguments vs Config Files

CLI arguments override config file values. This is useful for testing different configurations without modifying the config file.

### Package Manager

This project uses `pnpm` (v9.12.3+). The `packageManager` field is locked in `package.json`.

## Version Management Strategy

The project follows semantic versioning. The current version is in `package.json` (`version` field).

### For bumpp + GitHub Actions Integration

When implementing version bumping with `bumpp` and GitHub Actions:

1. **bumpp Configuration**: Create a `.bumpprc` or `bumpp.config.js` file, or use CLI options
2. **Version Update Files**: Ensure `bumpp` updates `package.json`
3. **GitHub Actions Workflow**: Should trigger on push to specific branches or manual workflow dispatch
4. **Publishing to GitHub Registry**: Configure `package.json` with:
   ```json
   {
     "publishConfig": {
       "registry": "https://npm.pkg.github.com"
     }
   }
   ```
5. **NPM Token**: Use `NODE_AUTH_TOKEN` environment variable in GitHub Actions with `GITHUB_TOKEN`
6. **Commit Strategy**: Configure bumpp to commit version changes and create git tags

### Migration Notes

From v0.4.x to v0.5.0: `googleapis` became a direct dependency (no longer peer dependency).
