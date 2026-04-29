# Cargo-Chef Warning Triage 2026-04-29

This report records the triage for Docker build warnings like:

```text
unused manifest key: bin.0.plugin
unused manifest key: test.0.plugin
unused manifest key: lib.plugin
```

## Result

- Current workspace manifests do not contain `plugin = ...` custom keys.
- `cargo metadata --no-deps --format-version 1` succeeds against the current
  workspace manifests without emitting those warnings.
- No `recipe.json` is checked into the repository.
- The warning therefore is not currently actionable as a direct Cargo.toml
  cleanup. It should be rechecked inside a stable Docker context with
  `cargo-chef` 0.1.72 and a fresh build cache.

## Commands

```bash
rg -n "plugin\s*=" . -g 'Cargo.toml' -g '*.toml' -g '*.json'
cargo metadata --no-deps --format-version 1
find . -name recipe.json -o -name '*.recipe.json' -o -path './target/*cargo*chef*'
```

## Assessment

The most likely source is cargo-chef recipe/skeleton generation or stale Docker
build cache, not checked-in workspace manifest metadata. Avoid adding
`package.metadata` or rewriting manifests until the warning is reproduced from a
fresh Docker build context.

## Follow-Up

- Re-run Docker release smoke from a stable Linux Docker context.
- If the warning persists with a fresh build cache, inspect the generated
  `/app/recipe.json` and cargo-chef skeleton manifests inside the builder stage.
- If the warning disappears, remove it from release follow-up as stale Docker
  cache noise.
