import { defineConfig } from 'tsdown'

export default defineConfig((options) => {
  return {
    entry: [
      'src/index.ts',
      '!src/**/__tests__/**',
      '!src/**/*.test.*',
      '!example',
    ],
    clean: true,
    format: ['esm', 'cjs'],
    external: ['googleapis'],
    noExternal: ['lodash-es'],
    minify: !options.watch,
    sourcemap: true,
    dts: true,
    shims: true,
  }
})
