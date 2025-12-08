# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.5.0...main

[compare changes](https://github.com/gn00678465/google-sheet-languages-model/compare/v0.5.0...main)

### 🏡 Chore

- 移除 credentials.json 的 TypeScript 錯誤註解 ([b7b71a8](https://github.com/gn00678465/google-sheet-languages-model/commit/b7b71a8))
- 移除 pnpm 設定中的版本指定 ([7307f15](https://github.com/gn00678465/google-sheet-languages-model/commit/7307f15))
- 更新 tsconfig.json 的 exclude 設定 ([2dcea5a](https://github.com/gn00678465/google-sheet-languages-model/commit/2dcea5a))
- **ci:** 移除 CI 流程中的 Node.js 版本矩陣設定 ([71d14b9](https://github.com/gn00678465/google-sheet-languages-model/commit/71d14b9))
- 更新 credentials.json 的 TypeScript 錯誤註解以提供更清晰的說明 ([c16950e](https://github.com/gn00678465/google-sheet-languages-model/commit/c16950e))

### ❤️ Contributors

- Madao <gn00678465@gmail.com>

## v0.5.0

### 🚀 Features
- Add CLI interface with pull/push commands
- Support config file for CLI (gslm.config.js)
- Support credentials as object or file path
- Add comprehensive validation for config and inputs

### 🐛 Bug Fixes
- Fix configuration object validation logic
- Fix language code validation regex
- Update path imports to use node:path and node:url

### 📚 Documentation
- Update README with CLI usage examples
- Add commit message guidelines

### ♻️ Refactors
- Refactor project structure and integrate Copilot settings
- Refactor config loading and merging logic

### 📦 Build System
- Migrate from tsup to tsdown

## v0.4.0

### 🚀 Features
- Initial release with programmatic API
- Support for Google Sheets integration
- Bidirectional sync (pull/push)
- Support for nested and flat i18n structures
