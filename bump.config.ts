import { defineConfig } from 'bumpp'

// Releases now target the napi package (packages/gslm). The tag prefix
// `napi-v` drives .github/workflows/napi.yml; the legacy `v*` flow is retired.
export default defineConfig({
  files: ['packages/gslm/package.json'],
  commit: 'chore(release): napi-v%s',
  tag: 'napi-v%s',
  push: true,
  confirm: true,
  // `all: true` commits whatever else is in the tree along with the version
  // bump — the changelog `execute` writes needs it. It also makes bumpp skip
  // its working-tree check, so only release from a clean tree.
  all: true,
  execute: 'pnpm run changelog',
  release: false,
})
