// Cross-boundary checks only: types and errors survive JS ⇄ Rust.
// The full edge-case matrix lives in crates/gslm-core.
const { describe, it } = require('node:test')
const assert = require('node:assert/strict')
const { unflatten, sheetToModel, modelToSheet, orphanKeys } = require('../index.js')

describe('unflatten', () => {
  it('rebuilds nesting and preserves order at every level', () => {
    const out = unflatten({ 'z.y': '1', 'z.x': '2', a: '3' })
    assert.deepEqual(out, { z: { y: '1', x: '2' }, a: '3' })
    assert.deepEqual(Object.keys(out), ['z', 'a'])
    assert.deepEqual(Object.keys(out.z), ['y', 'x'])
  })

  it('keeps empty strings and honours a custom separator', () => {
    assert.deepEqual(unflatten({ 'a/b': '' }, '/'), { a: { b: '' } })
  })

  it('throws an Error on key conflicts', () => {
    assert.throws(() => unflatten({ a: 'x', 'a.b': 'y' }), (e) => e instanceof Error && /conflicts/.test(e.message))
  })
})

describe('sheetToModel', () => {
  const rows = [
    ['key', 'zh-TW', 'notes', 'en'],
    ['user.name', '名字', 'ignored', 'Name'],
    ['ok', '', '', 'OK'],
    ['', 'spacer'],
  ]

  it('matches columns by header and returns plain data', () => {
    const m = sheetToModel(rows, ['en', 'zh-TW'])
    assert.deepEqual(m, {
      locales: ['en', 'zh-TW'],
      catalogs: {
        en: { 'user.name': 'Name', ok: 'OK' },
        'zh-TW': { 'user.name': '名字' },
      },
    })
    assert.deepEqual(Object.keys(m.catalogs), ['en', 'zh-TW'])
    assert.equal(JSON.parse(JSON.stringify(m)).locales[0], 'en')
  })

  it('throws listing available columns when a locale is missing', () => {
    assert.throws(() => sheetToModel(rows, ['fr']), /"fr" not found in header.*zh-TW.*notes.*en/)
  })

  it('throws on an empty sheet and on duplicate keys', () => {
    assert.throws(() => sheetToModel([], ['en']), /sheet is empty/)
    assert.throws(() => sheetToModel([['key', 'en'], ['a', '1'], ['a', '2']], ['en']), /duplicate key "a" at row 3/)
  })
})

describe('modelToSheet / orphanKeys', () => {
  const model = {
    locales: ['en', 'fr'],
    catalogs: { en: { a: 'A', b: '' }, fr: { z: 'Z', a: 'A-fr' } },
  }

  it('emits header, source order, then orphan keys; blanks for missing/empty', () => {
    assert.deepEqual(modelToSheet(model), [
      ['key', 'en', 'fr'],
      ['a', 'A', 'A-fr'],
      ['b', '', ''],
      ['z', '', 'Z'],
    ])
    assert.deepEqual(orphanKeys(model), ['z'])
  })

  it('round-trips through sheetToModel (empty strings become missing)', () => {
    const back = sheetToModel(modelToSheet(model), ['en', 'fr'])
    assert.deepEqual(back.catalogs, { en: { a: 'A' }, fr: { a: 'A-fr', z: 'Z' } })
  })

  it('throws when a catalog names a locale not in the list', () => {
    assert.throws(() => modelToSheet({ locales: ['en'], catalogs: { de: {} } }), /"de" is not part of this model/)
  })
})
