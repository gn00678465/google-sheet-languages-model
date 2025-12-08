import { defineConfig } from 'bumpp'

export default defineConfig({
  // Files to update version in
  files: [
    'package.json',
    'deno.json',
  ],

  // Commit settings
  commit: 'chore(release): bump version to v%s',
  tag: 'v%s',
  push: true,

  // Confirmation prompt
  confirm: true,

  // Execute changelog generation before committing
  execute: 'pnpm run changelog',

  // Don't create a release on GitHub (we'll do this via Actions)
  release: false,

  // Conventional commits support
  all: true,
})
