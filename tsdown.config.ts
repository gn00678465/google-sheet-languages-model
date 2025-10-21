import { defineConfig } from 'tsdown'

export default defineConfig((options) => {
  return {
    entry: {
      index: 'src/index.ts',
      cli: 'src/cli.ts',
    },
    clean: true,
    format: ['esm', 'cjs'],
    external: ['googleapis', 'yargs'],
    noExternal: ['lodash-es'],
    minify: !options.watch,
    sourcemap: true,
    dts: true,
    shims: true,
  }
})
