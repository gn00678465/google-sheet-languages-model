const SCHEMA_URL =
  'https://gn00678465.github.io/google-sheet-languages-model/schema/v1.json'

const LEGACY_FIELDS = new Set([
  'sheetId',
  'sheetTitle',
  'languages',
  'directory',
  'type',
  'credentials',
])

function tomlString(value) {
  return JSON.stringify(value)
}

function requiredString(config, field) {
  if (typeof config[field] !== 'string' || config[field] === '') {
    throw new TypeError(`舊設定缺少字串欄位 ${field}`)
  }
  return config[field]
}

function requiredLocales(config) {
  if (
    !Array.isArray(config.languages) ||
    config.languages.length === 0 ||
    config.languages.some((locale) => typeof locale !== 'string')
  ) {
    throw new TypeError('舊設定的 languages 必須是非空字串陣列')
  }
  return config.languages
}

function pathTemplate(directory) {
  const normalized = directory.replaceAll('\\', '/')
  return normalized.endsWith('/')
    ? `${normalized}{locale}.json`
    : `${normalized}/{locale}.json`
}

/**
 * Convert a loaded legacy config object to readable TOML without ever copying
 * inline Service Account credentials into the result.
 */
function migrateLegacyConfig(legacy) {
  if (!legacy || typeof legacy !== 'object' || Array.isArray(legacy)) {
    throw new TypeError('舊設定必須匯出一個物件')
  }

  const sheet = requiredString(legacy, 'sheetId')
  const tab = requiredString(legacy, 'sheetTitle')
  const locales = requiredLocales(legacy)
  const directory = requiredString(legacy, 'directory')
  const format = legacy.type === undefined ? 'nest' : legacy.type
  if (format !== 'nest' && format !== 'flat') {
    throw new TypeError('舊設定的 type 必須是 nest 或 flat')
  }

  const warnings = []
  const lines = [
    `#:schema ${SCHEMA_URL}`,
    '# 由 gslm migrate 產生；可依專案需求調整。',
    'version = 1',
    '',
    '# Google Sheet ID',
    `sheet = ${tomlString(sheet)}`,
    '# Tab 名稱',
    `tab = ${tomlString(tab)}`,
    '# 第一個 Locale 是 Source locale',
    `locales = [${locales.map(tomlString).join(', ')}]`,
    '# 本地 Catalog 路徑樣板',
    `path = ${tomlString(pathTemplate(directory))}`,
    '# Catalog 格式',
    `format = ${tomlString(format)}`,
  ]

  if (typeof legacy.credentials === 'string') {
    lines.push('', '[credentials]', `file = ${tomlString(legacy.credentials)}`)
  } else if (legacy.credentials && typeof legacy.credentials === 'object') {
    lines.push('', '[credentials]', 'file = "./credentials.json"')
    warnings.push(
      '舊設定內嵌憑證；請把金鑰另存為 ./credentials.json 並加入 .gitignore。',
    )
  } else if (legacy.credentials !== undefined && legacy.credentials !== null) {
    throw new TypeError('舊設定的 credentials 必須是字串或物件')
  } else {
    warnings.push(
      '未提供 credentials；新版將使用 Google Application Default Credentials（會先讀 GOOGLE_APPLICATION_CREDENTIALS）。',
    )
  }

  for (const field of Object.keys(legacy)) {
    if (!LEGACY_FIELDS.has(field)) {
      warnings.push(`略過不支援的舊欄位：${field}。`)
    }
  }

  return { toml: `${lines.join('\n')}\n`, warnings }
}

module.exports = { SCHEMA_URL, migrateLegacyConfig }
