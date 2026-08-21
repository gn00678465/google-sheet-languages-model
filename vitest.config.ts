import { defineConfig } from 'vitest/config'

// Root config covers the legacy TypeScript implementation only.
// The napi package has its own config under packages/gslm.
export default defineConfig({
  test: {
    include: ['src/**/*.test.ts'],
  },
})
