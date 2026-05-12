# Release Delivery Smoke - 2026-05-12

本报告记录 `feature-acceptance-gap-scan-2026-05-12-02.md` 指定的 release delivery smoke refresh。`docs/plan/` 仍是唯一权威；本文件只记录执行结果。

## Scope

- Web release build
- Embedded frontend runtime delivery
- Public runtime release shape endpoint
- Docker production-auth image smoke

## Verification

已运行：

- `scripts/smoke-web-release-build.sh`
- `DEVE_RUNTIME_BASE_URL=http://127.0.0.1:3101 DEVE_RUNTIME_SMOKE_REQUIRED=1 scripts/smoke-runtime-release-info.sh`
- `DEVE_DOCKER_SMOKE_REQUIRED=1 DEVE_DOCKER_SMOKE_PORT=3102 scripts/smoke-docker-release.sh`

## Results

### R1. Web Release Build

结果：通过。

要点：

- Trunk release build 成功。
- `apps/web/dist` 重新生成并由 embedded frontend 路径消费。
- 本次构建未留下需提交的 generated asset diff。

### R2. Runtime Release Info

结果：通过。

运行方式：

- 使用隔离数据根 `/tmp/deve-release-smoke-*`。
- 以 `serve --dev --port 3101` 启动本地后端。
- 使用 `DEVE_LEDGER_DIR` 与 `DEVE_VAULT_PATH` 隔离 ledger / vault。

观测摘要：

- `/api/node/role` 返回 `main v0.0.1 standard embedded-frontend development`。
- `repo_health` 为 `healthy(0/1)`。
- 临时后端与临时数据根已清理。

### R3. Docker Production Auth Smoke

结果：通过。

运行方式：

- `DEVE_DOCKER_SMOKE_REQUIRED=1`
- `DEVE_DOCKER_SMOKE_PORT=3102`
- 镜像：`deve-notebook:local-smoke`

要点：

- Dockerfile 本地 build 成功。
- 容器以 production auth material 启动。
- 宿主机 probe `http://127.0.0.1:3102/api/node/role` 返回 200。
- smoke container 与临时数据目录由脚本清理。

## Next

Release delivery 当前无 blocking drift。下一步回到 feature / acceptance / code 交叉扫描，选择下一批用户可感知实现项。
