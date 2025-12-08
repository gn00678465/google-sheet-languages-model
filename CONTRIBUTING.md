# Contributing to google-sheet-languages-model

Thank you for considering contributing to this project!

## Development Setup

1. Fork and clone the repository
   ```bash
   git clone https://github.com/gn00678465/google-sheet-languages-model.git
   cd google-sheet-languages-model
   ```

2. Install dependencies
   ```bash
   pnpm install
   ```

3. Make your changes

4. Run tests
   ```bash
   pnpm test
   ```

5. Build the project
   ```bash
   pnpm build
   ```

6. Submit a pull request

## Commit Messages

This project follows [Conventional Commits](https://www.conventionalcommits.org/).

### Format

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `refactor`: Code refactoring
- `test`: Adding tests
- `chore`: Maintenance
- `build`: Build system
- `ci`: CI/CD changes
- `perf`: Performance improvements

### Examples

- `feat(cli): add JSON output format`
- `fix(auth): handle expired credentials`
- `docs: update installation instructions`
- `refactor(core): simplify error handling`
- `test(validator): add edge case tests`

### Breaking Changes

If your change introduces a breaking change, add `!` after the type or include `BREAKING CHANGE:` in the footer:

```
feat!: change API response format

BREAKING CHANGE: The API now returns data in a different format.
Previous format: { data: [...] }
New format: { items: [...] }
```

## Testing

Before submitting a PR, ensure all tests pass:

```bash
# Run all tests
pnpm test

# Watch mode during development
pnpm test:watch

# Type checking
pnpm typecheck
```

## Code Style

- Follow the existing code style
- Use TypeScript for all new code
- Add JSDoc comments for public APIs
- Keep functions small and focused
- Write descriptive variable names

## Pull Request Process

1. Update the README.md with details of changes if applicable
2. Ensure all tests pass and type checking succeeds
3. Update examples if you've changed the API
4. The PR will be merged once it has been reviewed and approved

## Release Process (for Maintainers)

Maintainers can create releases using the following commands:

```bash
# Patch version (0.5.0 -> 0.5.1) - for bug fixes
pnpm release

# Minor version (0.5.0 -> 0.6.0) - for new features
pnpm release:minor

# Major version (0.5.0 -> 1.0.0) - for breaking changes
pnpm release:major
```

This will:
1. Update version in `package.json` and `deno.json`
2. Generate/update `CHANGELOG.md`
3. Create a git commit with the changes
4. Create a git tag
5. Push to GitHub
6. Trigger the automated publishing workflow

The GitHub Actions workflow will then:
- Run type checking and tests
- Build the package
- Publish to GitHub Package Registry
- Create a GitHub Release with the changelog

## Questions?

If you have any questions, feel free to open an issue for discussion.
