import { defineConfig } from 'bumpp'

// Releases now target the napi package (packages/gslm). The tag prefix
// `napi-v` drives .github/workflows/napi.yml; the legacy `v*` flow is retired.
export default defineConfig({
  files: ['packages/gslm/package.json'],
  commit: 'chore(release): napi-v%s',
  tag: 'napi-v%s',
  push: true,
  confirm: true,
  execute: 'pnpm run changelog',
  release: false,
  all: true,
})
