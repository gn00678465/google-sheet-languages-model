# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
