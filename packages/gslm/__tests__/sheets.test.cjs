// Drives SheetsClient end-to-end against a local node:http fixture server,
// using a static access token. Proves the napi async path (tokio runtime
// inside Node) on every CI platform without touching Google.
const { describe, it, before, after } = require('node:test')
const assert = require('node:assert/strict')
const http = require('node:http')
const { SheetsClient, sheetToModel } = require('../index.js')

const SHEET = '1AbC-def_GHI'
let server, baseUrl
const requests = []
let nextResponse = null // { status, body } override for the next request

before(async () => {
  server = http.createServer((req, res) => {
    let body = ''
    req.on('data', (c) => (body += c))
    req.on('end', () => {
      requests.push({ method: req.method, url: req.url, auth: req.headers.authorization, body })
      if (nextResponse) {
        const { status, body: b } = nextResponse
        nextResponse = null
        res.writeHead(status, { 'content-type': 'application/json' })
        return res.end(JSON.stringify(b))
      }
      res.writeHead(200, { 'content-type': 'application/json' })
      if (req.method === 'GET') {
        return res.end(JSON.stringify({ values: [['key', 'en', 'zh'], ['ok', 'OK', '好'], ['short', 'S']] }))
      }
      res.end(JSON.stringify({}))
    })
  })
  await new Promise((r) => server.listen(0, '127.0.0.1', r))
  baseUrl = `http://127.0.0.1:${server.address().port}`
})

after(() => server.close())

const googleError = (status, message) => ({ status, body: { error: { code: status, message, status: 'X' } } })

describe('SheetsClient', () => {
  it('create rejects conflicting or invalid credentials with code CREDENTIALS', async () => {
    await assert.rejects(
      SheetsClient.create({ credentials: { file: 'a', json: 'b' } }),
      (e) => e.code === 'CREDENTIALS' && /only one of/.test(e.message) && !/^\[/.test(e.message),
    )
    await assert.rejects(
      SheetsClient.create({ credentials: { file: '/nonexistent/sa.json' } }),
      (e) => e.code === 'CREDENTIALS' && /nonexistent/.test(e.message),
    )
  })

  it('readTab returns string[][] and sends a bearer token to the encoded range', async () => {
    requests.length = 0
    const client = await SheetsClient.create({ credentials: { accessToken: 'tok' }, baseUrl })
    const rows = await client.readTab(SHEET, "it's 翻譯")
    assert.deepEqual(rows, [['key', 'en', 'zh'], ['ok', 'OK', '好'], ['short', 'S']])
    assert.equal(requests.length, 1)
    assert.equal(requests[0].method, 'GET')
    assert.equal(requests[0].auth, 'Bearer tok')
    assert.match(requests[0].url, /\/v4\/spreadsheets\/1AbC-def_GHI\/values\/%27it%27%27s%20%E7%BF%BB%E8%AD%AF%27\?/)
    assert.match(requests[0].url, /valueRenderOption=FORMATTED_VALUE/)

    // chains straight into the core conversion
    const model = sheetToModel(rows, ['en', 'zh'])
    assert.deepEqual(model.catalogs.zh, { ok: '好' })
  })

  it('writeTab clears then updates with RAW', async () => {
    requests.length = 0
    const client = await SheetsClient.create({ credentials: { accessToken: 'tok' }, baseUrl })
    await client.writeTab(SHEET, 'i18n', [['key', 'en'], ['ok', '=SUM(1)']])
    assert.deepEqual(requests.map((r) => r.method), ['POST', 'PUT'])
    assert.match(requests[0].url, /%27i18n%27:clear$/)
    assert.match(requests[1].url, /%27i18n%27%21A1\?valueInputOption=RAW$/)
    assert.deepEqual(JSON.parse(requests[1].body), {
      range: "'i18n'!A1",
      majorDimension: 'ROWS',
      values: [['key', 'en'], ['ok', '=SUM(1)']],
    })
  })

  it('maps API errors to codes and strips the prefix from the message', async () => {
    const client = await SheetsClient.create({ credentials: { accessToken: 'tok' }, baseUrl })
    nextResponse = googleError(403, 'The caller does not have permission')
    await assert.rejects(client.readTab(SHEET, 't'), (e) => {
      assert.equal(e.code, 'PERMISSION_DENIED')
      assert.match(e.message, /^permission denied for sheet 1AbC-def_GHI: share the sheet with/)
      return e instanceof Error
    })
    nextResponse = googleError(400, "Unable to parse range: 't'")
    await assert.rejects(client.readTab(SHEET, 't'), (e) => e.code === 'TAB_NOT_FOUND')
    nextResponse = googleError(429, 'Quota exceeded')
    await assert.rejects(client.readTab(SHEET, 't'), (e) => e.code === 'RATE_LIMITED')
  })

  it('reports NETWORK when nothing listens', async () => {
    const client = await SheetsClient.create({ credentials: { accessToken: 'tok' }, baseUrl: 'http://127.0.0.1:1' })
    await assert.rejects(client.readTab(SHEET, 't'), (e) => e.code === 'NETWORK')
  })
})
