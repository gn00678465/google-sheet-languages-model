import { defineConfig } from 'tsup'

export default defineConfig((options) => {
  return {
    entry: [
      'src/index.ts',
      '!src/**/__tests__/**',
      '!src/**/*.test.*',
      '!example',
    ],
    clean: true,
    format: ['esm'],
    minify: !options.watch,
    sourcemap: true,
    dts: true,
    outExtension({ format }) {
      return {
        js: `.${format}.js`,
      }
    },
  }
})
