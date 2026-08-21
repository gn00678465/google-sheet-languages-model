// Uses node:test so the suite runs in any container without node_modules
// (vitest/esbuild ship platform-specific binaries that break across libc/arch).
const { describe, it } = require('node:test')
const assert = require('node:assert/strict')
const { flatten, version } = require('../index.js')

describe('flatten', () => {
  it('flattens nested objects with "." and preserves key order', () => {
    const out = flatten({ a: { b: 'x' }, c: 'y' })
    assert.deepEqual(out, { 'a.b': 'x', c: 'y' })
    assert.deepEqual(Object.keys(out), ['a.b', 'c'])
  })

  it('preserves depth-first order across deep nesting', () => {
    const out = flatten({ z: { y: { x: '1' }, w: '2' }, a: '3' })
    assert.deepEqual(Object.keys(out), ['z.y.x', 'z.w', 'a'])
  })

  it('accepts a custom separator', () => {
    assert.deepEqual(flatten({ a: { b: 'x' } }, '/'), { 'a/b': 'x' })
  })

  it('returns an empty object for an empty object', () => {
    assert.deepEqual(flatten({}), {})
  })

  it('throws an Error for numeric key segments', () => {
    assert.throws(() => flatten({ a: { 0: 'x' } }), (e) => e instanceof Error && /must not be a number/.test(e.message))
  })

  it('throws for non-object input', () => {
    assert.throws(() => flatten(['a']), /expected an object/)
    assert.throws(() => flatten('s'), /expected an object/)
  })

  it('throws for an empty separator', () => {
    assert.throws(() => flatten({ a: 'b' }, ''), /separator must not be empty/)
  })
})

describe('version', () => {
  it('returns a non-empty semver-like string', () => {
    assert.match(version(), /^\d+\.\d+\.\d+/)
  })
})
