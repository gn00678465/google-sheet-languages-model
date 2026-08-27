const SCHEMA_URL =
  'https://raw.githubusercontent.com/gn00678465/google-sheet-languages-model/main/docs/schema/v1.json'

const LEGACY_FIELDS = new Set([
  'sheetId',
  'sheetTitle',
  'languages',
  'directory',
  'type',
  'credentials',
])

function tomlBasicString(value) {
  let output = '"'
  for (let index = 0; index < value.length; index += 1) {
    const code = value.charCodeAt(index)
    if (code >= 0xd800 && code <= 0xdbff) {
      const next = value.charCodeAt(index + 1)
      if (!(next >= 0xdc00 && next <= 0xdfff)) {
        throw new TypeError('舊設定字串含無法表示為 TOML 的孤立 surrogate')
      }
      output += value[index] + value[index + 1]
      index += 1
      continue
    }
    if (code >= 0xdc00 && code <= 0xdfff) {
      throw new TypeError('舊設定字串含無法表示為 TOML 的孤立 surrogate')
    }
    if (code === 0x22) output += '\\"'
    else if (code === 0x5c) output += '\\\\'
    else if (code === 0x08) output += '\\b'
    else if (code === 0x09) output += '\\t'
    else if (code === 0x0a) output += '\\n'
    else if (code === 0x0c) output += '\\f'
    else if (code === 0x0d) output += '\\r'
    else if (code < 0x20 || code === 0x7f) output += `\\u${code.toString(16).padStart(4, '0')}`
    else output += value[index]
  }
  return `${output}"`
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
    config.languages.some((locale) => typeof locale !== 'string' || locale === '')
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
    `sheet = ${tomlBasicString(sheet)}`,
    '# Tab 名稱',
    `tab = ${tomlBasicString(tab)}`,
    '# 第一個 Locale 是 Source locale',
    `locales = [${locales.map(tomlBasicString).join(', ')}]`,
    '# 本地 Catalog 路徑樣板',
    `path = ${tomlBasicString(pathTemplate(directory))}`,
    '# Catalog 格式',
    `format = ${tomlBasicString(format)}`,
  ]

  if (typeof legacy.credentials === 'string') {
    lines.push('', '[credentials]', `file = ${tomlBasicString(legacy.credentials)}`)
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
