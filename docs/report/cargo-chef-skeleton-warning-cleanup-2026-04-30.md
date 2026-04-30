# Cargo-Chef Skeleton Warning Cleanup 2026-04-30

This report closes the Docker-only cargo-chef warning follow-up from the full
workspace verification pass.

## Result

- Blocking failures: 0.
- The checked-in workspace manifests remain clean: the warning does not come
  from repository `Cargo.toml` files.
- The warning source is cargo-chef 0.1.72 recipe/skeleton generation. The
  generated `/app/recipe.json` embeds target-level `plugin = false` lines in
  synthetic manifests, which Cargo reports as unknown keys during
  `cargo chef cook`.
- The Dockerfile now strips only the generated `\nplugin = false` recipe noise
  before `cargo chef cook`. It does not rewrite checked-in manifests and does
  not change product runtime behavior.

## Verified

```bash
docker build --target planner -t deve-notebook:planner-inspect .
docker create --name deve-chef-planner-inspect deve-notebook:planner-inspect
docker cp deve-chef-planner-inspect:/app/recipe.json /tmp/deve-cargo-chef-recipe.json
docker rm deve-chef-planner-inspect
rg -n '"plugin"|plugin\s*=|bin\.0\.plugin|lib\.plugin|test\.0\.plugin' /tmp/deve-cargo-chef-recipe.json
cp /tmp/deve-cargo-chef-recipe.json /tmp/deve-cargo-chef-recipe-sanitized.json
sed -i 's/\\nplugin = false//g' /tmp/deve-cargo-chef-recipe-sanitized.json
python3 -m json.tool /tmp/deve-cargo-chef-recipe-sanitized.json
docker build --target deps -t deve-notebook:deps-warning-check .
DEVE_DOCKER_SMOKE_REQUIRED=1 DEVE_DOCKER_SMOKE_PORT=3102 scripts/smoke-docker-release.sh
```

The final Docker release smoke completed with:

```text
docker-release-smoke: ok
```

## Regression Guard

`scripts/check-release-baseline.sh` now asserts that the Dockerfile keeps the
recipe sanitization line immediately before the `cargo chef cook` release layer.

## Follow-Up

No active follow-up remains for this warning family unless a future cargo-chef
upgrade removes the generated `plugin = false` keys. At that point the
sanitization line can be deleted with a fresh Docker release smoke.
