# Watcher Lifecycle Duplicate Start Gate

日期：2026-05-14

## Scope

本批次执行 `Mainline implementation gap scan` 后选择 `04_storage.md#watcher-contract` 的 watcher lifecycle 小闭环：同一 repo 不得同时存在多个 watcher，重复启动失败时不得留下未登记 runtime。

## Finding

`start_repo_watcher` 原先在线程启动后才写入 watcher registry。若同一 repo 已有 watcher，registry 会返回 `AlreadyRunning`，但新建线程已经存在，存在 orphan watcher 风险。

## Fix

- watcher 启动前先通过 registry 检查同 repo 是否已运行。
- registry 登记改为 `insert_or_reject`，在并发竞态下仍返回 rejected handle。
- rejected watcher handle 会被显式 `stop + join`，避免未登记线程继续消费事件。
- `stop_repo_watcher` 复用统一 `stop_handle`，保持 close/drain 语义集中。
- `STORE-007` 增加 duplicate watcher lifecycle 验收测试与 baseline guard。

## Verification

- `cargo test -p deve_core duplicate_insert_rejects_second_handle_without_replacing_existing -- --nocapture`
- `cargo test -p deve_core watcher_duplicate_start_fails_and_can_restart_after_stop -- --nocapture`
- `cargo test -p deve_core watcher_records_create_modify_delete_candidates -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p deve_core --all-targets -- -D warnings`
- `bash scripts/check-storage-repo-baseline.sh`
- `bash scripts/plan-coverage.sh --summary-missing-plan-ref`

## Remaining

- 本批次不改变 watcher 服务粒度：当前 server 启动时仍为全部 healthy local repos 启动 watcher，Web repo switch 不是 repo close。
- 更大的 watcher lifecycle 设计项仍是：如果未来引入 repo mount/unmount runtime，需要明确 active-scope watcher 与 all-local-repo watcher 的切换边界。
