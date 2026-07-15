# 18_release.md - 发布与运维 (Release & Ops)

## Metadata

- `Layer`: `Peripheral / Deferred`
- `Status`: `Reference`
- `Version`: `0.0.1`
- `Last Review`: `2026-07-14`
- `Counterpart Feature`: `docs/features/15_release.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/12_tech_release.md`
- `Primary Code Areas`: `.github/workflows/`, `Dockerfile`, `scripts/`, `tools/baseline`

本章定义发布策略、版本规范与 CI/CD。

## 1. Distribution Strategy (分发策略)

分发渠道：

### 1.1 Support Matrix (支持矩阵)
| Platform    | Artifact Format             | Architecture         | Signing                  |
| :---------- | :-------------------------- | :------------------- | :----------------------- |
| **Windows** | `.msi`, `.nsis` (Setup)     | x64, ARM64           | EV Cert (Optional)       |
| **macOS**   | `.dmg`, `.app`              | Apple Silicon, Intel | **Required** (Notarized) |
| **Linux**   | `.deb`, `.rpm`, `.AppImage` | x64                  | GPG                      |
| **Server**  | OCI Image (Docker/Podman)   | x64, ARM64           | GHCR                     |
| **iOS**     | `.ipa` (App Store)          | ARM64                | **Pending** (Not urgent) |
| **Android** | `.apk` / `.aab`             | ARM64                | **Pending** (Not urgent) |
| **Web**     | PWA (Static)                | Universal            | HTTPS                    |
| **CLI**     | `deve_cli` binary           | Host target          | Unsigned public preview  |

First formal tag scope note: Linux native Desktop artifacts (`.deb`, `.rpm`,
`.AppImage`) are deferred until the native shell stack can move off the current
GTK3/WebKitGTK 4.x dependency line. The tracked TODO is to upgrade or replace
the Tauri/Wry Linux shell route with a maintained GTK4/WebKitGTK 6-compatible
stack, then refresh target-host package/startup evidence before re-enabling
Linux native artifacts in a public release. Until that TODO is closed, Linux
users are expected to use Web / Server / Docker delivery rather than a Linux
native Desktop package.

### 1.2 Release Channels (发布通道)
1.  **Public Preview (公开预览)**: tag `v0.y.z`，用于 pre-1.0 阶段的首批公开验证；必须通过当前 release gate，但不得声明 stable data compatibility、签名 native release、store readiness 或 physical-device readiness。
2.  **Stable (稳定版)**: tag `v1.0.0`，仅在 Milestone 完成且测试通过后发布；产物包括二进制与 Docker Image (`latest`, `v1.0.0`)；适用于生产环境。
3.  **Pre-release / Experimental (预发布 / 实验构建)**: tag `vX.Y.Z-rc.N` 或人工测试构建标识；按里程碑需要手动触发或本地构建；发布基线不要求独立 `nightly.yml` 工作流。

## 2. CI/CD Pipelines (自动化流程)

CI/CD 基于 GitHub Actions。

> [!NOTE]
> 首个公开 tag 的发布基线由 `.github/workflows/release.yml` 作为唯一
> `v*` tag orchestrator，并调用 reusable `.github/workflows/release-native.yml`。
> `nightly.yml` 与
> `speckit-sync-check.yml` 不属于权威 release / CI 要求，不构成总蓝图 drift。

### 2.1 Workflow: `release.yml`
*   **Trigger**: Push to tag `v*` (e.g., `v1.2.3`).
*   **Steps**:
    1.  **Tag Gate**: GitHub 的 `v*` glob 只负责触发；workflow 的第一个 step 必须按 SemVer 2.0.0（含可选 prerelease/build metadata）验证 `GITHUB_REF_NAME`，非 SemVer `v*` tag 必须在 checkout、build 或 publish 前 fail-closed。checkout 后必须把去掉单个前导 `v` 的 tag 与 workspace package version、Desktop Tauri version、Mobile Tauri version 做逐字节 exact compare；prerelease 与 build metadata 不得被归一化或忽略，任一不一致都必须在 build/publish 前失败。
    2.  **Quality Gates**: `cargo clippy --locked --all-targets -- -D warnings`, `scripts/plan-coverage.sh --write-report`, `scripts/check-architecture-registry.sh`, native boundary checks that do not build Linux GTK3 artifacts, graph baseline, and `cargo test --locked`. The native process adapter gate is scoped with `DEVE_NATIVE_PROCESS_ADAPTER_RUN_NATIVE_PACKAGING_TESTS=0` in `release.yml`, so it verifies no-Tauri/process authority boundaries without compiling native-packaging dependencies.
        Dependency audit belongs to this gate: `scripts/check-release-audit-gate.sh`
        **MUST** fail on cargo/npm vulnerabilities and **MUST** compare every
        non-vulnerability `cargo audit` warning with
        `docs/registry/release-audit-warning-registry.md`. Any unregistered,
        stale, or field-incomplete warning is release-gate drift. The registry
        row must include the advisory identifier (or synthetic `YANKED` for
        cargo-audit yanked warnings without a RustSec advisory id), crate,
        warning kind, rationale, replacement route, and whether first-tag
        readiness requires a separate USER decision or replacement before
        public tag.
    3.  **Docker Build**: Dockerfile frontend stage 先运行 `npm run build` 产出 editor assets，再运行 `trunk build --release` 产出 Leptos/WASM。release job 只构建一次本地 candidate image，并记录其 image ID。
    4.  **Embed Frontend**: Dockerfile backend stage 在 `cargo build --release --package deve_cli` 前复制 `apps/web/dist`，使 CLI build script 将前端静态资源嵌入二进制。
    5.  **Exact Image Smoke**: runtime/login smoke 与 Playwright 双客户端 smoke 必须复用同一 candidate image，禁止在 smoke 内重新 build；全部 smoke 后 image ID 必须仍与构建时一致。
    6.  **Docker Push**: 仅在 exact image smoke 成功后，才把 candidate image 赋予 version 与 `latest` tag 并 push。
        *   **Registry**: GHCR (`ghcr.io`).
        *   **Platforms**: 发布基线为 `linux/amd64`；`linux/arm64` 需要独立验证后再加入。
        *   **Tags**: `latest`, `v1.2.3` (与 Release Tag 同步).
        *   **Digest Verification**: push 后必须从 registry 解析 version 与 `latest`，两者 manifest digest 必须完全相同；不一致时 release job 失败。

`release-native.yml` 是 reusable native delivery track，不得独立监听 `v*` tag。
`release.yml` 的 quality gates 与 Docker publish 成功后才可调用它。Windows
MSI/NSIS、macOS DMG 与 Android ARM64 APK 的 build jobs 只能先上传 workflow
artifacts；三个必需 build jobs 全部成功后，单一 publish job 才可创建或更新一次
draft GitHub Release。publish job 必须按 allowlisted artifact names 下载，先验证三个容器内
总文件数恰为四，再验证恰好存在
Windows MSI、Windows NSIS、macOS DMG 与 Android ARM64 APK，拒绝缺失、重复 basename
或额外 artifact；上传后必须从 GitHub Release API 复核 asset manifest，完全相等后才可
把 draft 发布为公开 GitHub Release。任一 native build、manifest、upload 或复核步骤失败时
不得留下公开的 partial GitHub Release。Reusable workflow 只接收四个 Android signing
secrets，不得继承全部 repository / organization secrets。Windows/macOS public-preview artifacts 可以保持 unsigned，Android
仅在 keystore secrets 齐全时产生已签名 APK，否则只能上传明确标记、不可安装的
unsigned diagnostic artifact。该 workflow **MUST NOT** 构建 Linux
GTK3/WebKitGTK 4.x Desktop artifacts 或 iOS artifacts，也不得把 package artifact
存在性表述为 signing、notarization、store 或 physical-device readiness。

该 first-tag orchestrator 是最小收敛而非跨 registry 强事务：Docker version / latest
image 可以在 native track 完成前已发布。若随后 native build 或 publish verification
失败，workflow 必须以失败结束且不得留下公开 GitHub Release；draft 与已发布 GHCR tag
的保留、删除或重跑由 maintainer 显式恢复流程处理，不得把 partial delivery 报告为完整 release。
同一 tag 重跑时，asset upload 前只允许 GitHub Release 不存在或仍为 draft；若该 tag 已有
公开 Release，workflow 必须在修改任何公开资产前 fail-closed，等待 maintainer 显式恢复。

Web 与 CLI 仍由 `deve_cli` 主通道承载：Docker image 在构建 `deve_cli` 前嵌入
release Web assets，Desktop LocalBackend package 将同一 CLI 作为受控 sidecar。
在独立 standalone CLI upload workflow 被显式加入 plan 前，不要求重复上传另一套
CLI artifact。

First-tag tag orchestrator deliberately does **not** run Linux GTK3/WebKitGTK 4.x
native packaging, iOS package-build, installer, signing/notarization, store or
physical-device smoke gates. Android ARM64 package-build is a required job in the
reusable native delivery track; Android install, emulator/device and signing-readiness
smokes remain target-host / workflow-dispatch evidence surfaces rather than tag publish gates.

Linux native Desktop bundling has an additional first-tag TODO: it **MUST NOT**
ship a Linux GTK3/WebKitGTK 4.x native artifact for the first formal tag. Before
Linux native artifacts can be restored to the release set, the project must
either adopt a maintained Tauri/Wry GTK4/WebKitGTK 6 route or replace the Linux
native shell route with an equivalent maintained WebView stack, and then rerun
release audit, package build, startup, and native-session smoke evidence on the
Linux target host.

Desktop startup / native-session smoke MAY accept `DEVE_DESKTOP_PACKAGE_BUNDLES=exe`
as a target-host release-binary-only probe after `target/release/deve_desktop.exe`
and its sibling `deve_cli.exe` have been built. This selector MUST NOT be
accepted by package-build or installer smoke gates, and MUST NOT be used as
evidence for MSI/NSIS package readiness, install/uninstall readiness, signing
readiness, store readiness, or physical-device readiness.

Native 双模式属于运行时能力门禁，不属于签名/store/physical-device release readiness。发布或 target-host 证据可以声明：

- LocalBackend 本地后端 smoke 通过。
- RemoteBrowser HTTPS origin 壳层 smoke 通过。

但不得在未完成签名、store、physical-device 与长期后台同步验收前声明 Desktop/Mobile release ready。

Windows installer/package smoke 必须使用已安装 Desktop bundle 及其 sidecar，并以临时
Projection Workspace 与本地 bare Git remote 验证 NoteGit commit、mirror、import/export
和 push；不得依赖公网 remote。该 smoke 还必须证明 Git 不存在时 LocalBackend 仍能启动，
且已成立的 NoteGit commit 不会因 mirror unavailable 被回滚。

Windows packaged UI evidence 必须独立于快速 startup marker probe。它使用已安装的 Desktop
可执行文件、临时 `DEVE_DESKTOP_DATA_DIR`、隔离 WebView2 user-data 与随机 CDP 端口，
通过真实 native WebView 完成 native session、创建文档、编辑、NoteGit commit/history 与
Settings 焦点约束，并在关闭窗口后证明 sidecar 无孤儿进程。该 CDP automation 只操作已安装
壳层所承载的 Web UI，不授予脚本或 Desktop shell 任何 ledger / Source Control authority。

Desktop RemoteBrowser target-host evidence 必须使用 host-local remote preference（不得依赖
CLI/env override）启动已安装壳层，并证明：远端 HTTPS 页面可登录、编辑、commit/history；
不存在 `ipc.localhost` 请求、CSP 错误、native bootstrap capability 或可调用 Tauri handler；
原生菜单/托盘切回 local 后进程重启并取得全新的 endpoint/session/scope；切换前后无孤儿
sidecar，旧 remote authority 不得复用。服务端 `connect-src` CSP 不得为该 smoke 放宽。

Android writable target-host receipt 必须同时记录 Android API level、当前 WebView provider
包名/完整版本、AVD/system-image 或真实设备标识，以及 non-extractable Ed25519 probe 结果。
正式支持基线为 API 29+ / provider major 137+；低于基线或 probe 失败只能生成不满足
tag-ready 的只读/unsupported evidence。不得引入 native crypto bridge、WASM fallback 或
软件密钥降级来满足该 receipt。

### 2.1.1 Developer Baseline Checkers {#developer-baseline-checkers}

发布与验收基线可以提供 Rust developer CLI mirror，用于替代对 host bash/awk/rg runtime 敏感的纯文本合同检查。该入口由独立 workspace tool crate `tools/baseline`（package `deve_baseline`）承载，属于 developer/release tooling，不属于普通用户 `deve` 命令面；它 **MUST NOT** 依赖 `deve_cli` 产品 runtime，默认也 **MUST NOT** 依赖 `deve_core`，更 **MUST NOT** 获得 ledger、projection、source-control 或 native authority 写权限。

Rust baseline checker 的默认聚合入口 `cargo run -p deve_baseline -- all` 只承载确定性的仓库文件检查：固定字符串存在/缺失、顺序检查、验收 case block 绑定、协议/文档常量钉扎，以及 `Cargo.lock` tracked / not ignored 这类轻量 git baseline。对于历史上已经作为 baseline shell 存在的确定性 `cargo test` 调度脚本，Rust checker MAY 提供 `cargo_test` TSV operation，并由 `cargo run -p deve_baseline -- full` 显式执行；该入口仍属于 developer/release tooling，不得启动产品 server、Docker/native packaging、平台 smoke、外部工具安装或 network runtime。Docker/native packaging、平台 smoke、外部工具安装与 network runtime 检查在未被显式建模前仍由 shell 脚本或 CI job 承担。Rust mirror 也 MAY 承载验收用例中已有的确定性边界检查脚本（例如结构化 WS 错误、browser prefs 边界、source-control smoke hygiene），前提是检查内容仍能表达为仓库文件合同而非运行时 smoke。Rust mirror 与 shell script 并存期间，确定性规格的唯一维护位置是 Rust checker 的 TSV spec；同名 shell 脚本只能作为兼容入口转发到 Rust checker，并输出相同风格的 fail-closed 诊断（`<name>-baseline-check: ...` 或既有脚本标签），避免 Windows/WSL bash runtime 不可用时失去本地验收入口，也避免长期双份规格漂移。

### 2.1.2 Validation Script Ownership

test / check / smoke 脚本的收敛目标是“验证逻辑尽可能由 Rust/CLI 拥有”，不是机械删除所有 shell 文件。新增或迁移脚本时必须先分类：

1.  **必须 Rust 化**：固定文本/文件合同检查、acceptance binding、registry、路径漂移、结构化错误、边界守卫、env 参数合法性、target 列表与 fail-closed 前置条件。这类检查 SHOULD 进入 `tools/baseline`；同名 shell 只能作为兼容 wrapper 调用 `run_deve_baseline`。
2.  **优先 Rust 化但允许 shell 编排**：`check-*-preflight.sh`、`check-local-quick-gate.sh`、`check-deep-audit-gate.sh`、`check-release-audit-gate.sh` 这类聚合或 preflight 入口。Rust/CLI SHOULD 拥有分类、参数校验、诊断格式与 fail-closed 判断；shell MAY 保留外部命令串联、CI glue 与宿主工具调用。
3.  **暂不强行纯 Rust**：Docker smoke、runtime server/browser smoke、adb/xcrun/installer/native package build、GitHub workflow dispatch 与 artifact collect。此类脚本 MAY 增加 Rust/CLI 前置校验或报告规范，但真实平台动作仍可由 shell/CI 编排；它们 MUST NOT 被并入 `deve_baseline -- all` 的轻量确定性聚合。

任何迁移不得形成双份长期规格：确定性规则的唯一维护位置应是 Rust checker / TSV spec；shell wrapper 不得复制同一批固定字符串、路径漂移或边界判定。

### 2.1.3 Workflow: `check.yml`

普通 branch push / pull request 可以运行一个 **check-only** CI workflow，用于在 tag release 前尽早发现格式、Rust 编译、WASM 编译、测试与文档合同漂移。该 workflow 不属于发布通道，也不得替代 `release.yml` 的 tag-triggered 发布基线。

`check.yml` **MUST** 保持以下边界：

- Trigger 仅限 branch push / pull request / 可选手动诊断；不得监听 `v*` tag。
- Permissions 只允许 `contents: read`；不得声明 `packages: write`。
- 不得登录 GHCR、不得执行 Docker build/push、不得 upload release artifact。
- 不得运行 native package build、installer smoke、store distribution、physical-device 或 production deploy。
- MAY 运行 `cargo fmt --check`、`cargo clippy --locked --all-targets -- -D warnings`、`cargo check --locked -p deve_web --target wasm32-unknown-unknown`、`cargo test --locked`、`cargo run -p deve_baseline -- all` 与 plan coverage enforcing checks。

### 2.1.4 First-tag Acceptance Matrix {#first-tag-acceptance-matrix}

`docs/registry/acceptance-matrix.tsv` 是验收需求与证据需求的唯一人工维护注册表；
`docs/acceptance-matrix.md` 只能由 checker 确定性生成，不是第二套 authority。旧的
`docs/acceptance-bindings.tsv` 在完整迁移后删除。矩阵字段固定为：

`requirement_id | journey_id | flow_id | case_id | surface | mode | gate | requirement | evidence_kind | evidence_id | evidence_ref | freshness | note`

- `requirement` 只允许 `required`、`conditional`、`non-goal`；后两者必须在 `note`
  给出具体边界或原因。
- `gate` 只允许 `ci`、`release`、`tag-ready`、`advisory`。
- `freshness` 只允许 `source-bound`、`current-head`、`target-host-30d`、
  `first-tag-once`、`none`。
- `case_id` 或 `flow_id` 没有对应对象时使用受控哨兵 `none`。每个 acceptance case
  至少映射一次；`20_operations_catalog` / `operation-coverage` 中每一条 flow-case 关系
  都必须在矩阵中出现，避免仅凭“文本中曾被提及”冒充验收绑定。
- `requirement_id` 是全表唯一 requirement key；同一 `evidence_id` 可以覆盖多条需求。
- `evidence_kind` 只允许 `source-ref`、`test`、`script`、`document`、`receipt`、
  `external-state`、`gap`。路径、脚本和 test selector 必须可解析；Rust test selector
  必须绑定真实 workspace package、可解析 test target，且 filter 必须匹配该 package/target
  源码中定义的测试函数。
  `receipt` locator 必须是规范相对 JSON 路径；`gap` 必须明确说明缺失事实，且不得满足
  `tag-ready`。

first-tag journey 集合固定覆盖：`auth-session`、`repo-lifecycle`、
`edit-sync-offline-recovery`、`source-control`、`external-changes`、`notegit`、
`p2p-gap-recovery`、`docker-multiclient`、`desktop-local-backend`、
`desktop-remote-browser`、`android-local-backend`、`android-remote-browser`（含 native-owned
`Use Local Backend` 恢复、新 endpoint/session/scope 与零 RemoteBrowser IPC）、
`release-artifacts`、`security-supply-chain`。矩阵必须为这些 journey 的适用 surface/mode
登记 `tag-ready/required` 需求；iOS target-host 仍为 `advisory/conditional`，必须如实说明
目标宿主与证据缺口，不能伪装为现有证据。

`deve_baseline acceptance-matrix` 属于普通 CI 的结构 gate，负责 case、flow、journey、
枚举、唯一键、rationale 与 evidence locator 完整性；它不把过期或尚未采集的运行时
证据伪装为通过。默认结构检查还必须验证生成内容与 `docs/acceptance-matrix.md`
一致；使用 `acceptance-matrix --render` 才能显式刷新该投影。

`deve_baseline acceptance-receipt --evidence-id <id> --evidence-ref <relative-json>
--surface <surface> --mode <mode> --target-os <target> --output <file>
[--claims <producer-json>] -- <command...>`
包装真实命令，并记录 receipt schema、evidence ID/ref、命令前后 HEAD/dirty、host OS/arch、
target OS、surface/mode、开始/结束 UTC 时间、退出状态、稳定命令指纹与脚本 artifact。该低层命令
生成的 receipt 明确标记为 `manual.unbound`，只用于开发诊断，不能满足 tag-ready；正式候选证据
必须由 `acceptance-run` 按 producer registry 生成。只有命令成功、
前后 HEAD 相同且 worktree 始终 clean 时才写 `passed`；其他情况仍必须原子写出 `failed`
receipt 并返回非零。Receipt output 必须位于 Git worktree 外，且其尾部路径必须与
`evidence_ref` 一致，避免 evidence 文件本身污染被测工作树。

Android writable receipt 必须使用 schema 3 `--claims`，并绑定到对应受控 smoke producer；claims
至少包含 SDK level、current WebView provider 完整版本、API 29+/WebView 137+ 判定、真实
non-extractable Ed25519 WebCrypto probe，以及 writable lifecycle 闭环结果。`tag-ready` 不接受
readonly-negative、瞬态 probe failure、任意 exit-0 命令或未绑定 producer 的 Android receipt。

`deve_baseline acceptance-matrix --tag-ready <receipt-dir>` 只接受矩阵中 `required` 且
`gate=tag-ready` 的新鲜证据：receipt 必须 `passed`、commit SHA 等于当前 HEAD、平台匹配、
worktree clean，并满足 `current-head` / `target-host-30d` / `first-tag-once` 的时效语义。
`source-bound` 由结构 checker 在当前源码上验证；`external-state` 与 `gap` 不得无 receipt
地满足 tag-ready。普通 CI 只阻断结构漂移；正式 tag workflow 必须汇总各平台 receipts
后再运行 tag-ready。

`docs/registry/acceptance-producers.json` 是 receipt producer 的唯一人工维护注册表。
它只登记 producer ID、覆盖的 `evidence_id`、执行层级、适用 host OS、超时、必需环境变量、
claims 输出变量、可公开且非凭据的 bound environment、受控 artifact 清单，以及由 `program + args[]` 组成的命令步骤；不得保存 shell command string，
不得在 JSON 中拼接凭据，也不得把普通文档/source reference 冒充运行时 producer。矩阵中每个
`tag-ready/required/receipt` evidence 必须恰好由一个 producer 覆盖；producer 也不得引用矩阵
之外或非 receipt 的 evidence。

`deve_baseline acceptance-run --tier <ci|full|target-host|tag-ready> --plan`
只做确定性解析与预检，输出将运行、因 host 不匹配而不可运行、缺少环境变量或不能满足
tag-ready host 约束的 producer，不启动外部命令。去掉 `--plan` 后必须显式提供位于 worktree
外的 `--receipt-dir`；runner 默认按 producer ID 顺序串行执行，低内存宿主不得由工具隐式并发。
`--producer <id>` / `--evidence-id <id>` 只允许缩小 producer 执行面；evidence filter 只选择其
owner producer，不得切开该 producer 的原子 evidence group。一次 producer 执行可以覆盖多个
evidence，但命令只能运行一次；每个 evidence 仍获得独立 locator、surface、mode 与 target OS
绑定的 schema 3 receipt。每组 receipt 还必须绑定 producer ID、当前 registry contract 指纹、唯一
execution ID、该次执行的完整 evidence 集合与受控 artifact 清单。多 evidence producer 必须先完成
全部序列化与临时写入再发布；单次原子 execution group 最多 64 个 receipt，任一 sibling 的 claims、
构造或资源预算失败必须令整组一致 failed。进程中断留下的部分集合不能通过 collector 或 tag-ready。`ci` 层只执行快速、确定性的 host-local producer；已有 workflow
拥有的 fmt/clippy/workspace test 不得为制造 receipt 而重复运行。`full` 增加 Docker/browser
业务闭环，`target-host` 选择当前宿主的 native/mobile producers，`tag-ready` 用于候选证据生产
与跨平台缺口预检，不能把单一宿主误报为覆盖所有平台。

Rust runner 独占以下 infra：参数/registry 校验、HEAD/dirty 前后快照、单调超时、子进程终止、
失败 receipt、producer claims 读取、命令指纹、execution group 与 receipt 发布。命令超时后
必须有界终止进程树并继续执行显式 finally steps；runner 不依赖被强杀 shell 的 trap 完成清理。
runner 为每次 producer 执行提供隔离的临时 state directory，使 finally step 只能回收本次执行
登记的宿主资源。Android 外部 finalizer 只能在 emulator serial 与登记 AVD 精确匹配后请求
设备退出；owner file 中的 PID 只用于有界观察资源消失，不能授权外部脚本按裸 PID 发送信号。
只有实际启动 emulator 的 shell 仍能证明该 PID 是自己的活动后台 job 时，才可在自身 EXIT trap
内发送信号。reserved launch 无法取得可验证设备身份时必须 fail-closed 并保留诊断，不能宣称清理成功。
Docker、Playwright、ADB、
WebView CDP、Tauri installer 与签名等宿主动作仍可由窄 shell/PowerShell/Node 脚本承载；这些
脚本不得重新实现矩阵选择、receipt schema、freshness 或聚合判断。Android producer 可在
Windows host 驱动 emulator，但 receipt 的 target OS 仍为 `android`，且必须通过实际 provider
与 Ed25519 probe，而不是根据宿主或版本号推断可写。

`deve_baseline acceptance-collect --output <receipt-root> <artifact-root>...` 负责 pre-publish
聚合：只接收普通 JSON 文件，拒绝 symlink/reparse escape、重复 `evidence_id`、重复 locator、
不完整或混合 execution group、非规范相对路径和越界目录；枚举时固定 canonical root，读取
前后必须重验同一 root identity。单个 receipt JSON 上限 1 MiB，单次聚合最多 4096 个文件且
JSON 总量上限 16 MiB；producer 写入侧单个原子 execution group 最多 64 个 evidence，claims
读取、receipt 序列化与临时发布仍应用 1 MiB 单文件、16 MiB 整组预算；跨 producer 的 collector
总文件上限仍为 4096。超限时生成有界 failed receipt 或在执行前拒绝，
不得先无界分配再交由 collector 拒绝。execution group 必须统一验证 HEAD/host/timestamps/status、command
fingerprint、artifacts 与 producer inputs。collector 使用临时目录完成全部校验后再原子发布。release workflow 必须先
下载各平台 receipt artifact、调用 collector，再运行 `acceptance-matrix --tag-ready`，并且该
gate 必须位于任何 image tag/push、Release asset upload 或 GitHub Release 创建之前。producer
失败、缺失、过期或平台不匹配均保持 fail-closed，禁止用空目录或跳过 job 伪装成功。

当前 first-tag 必须如实保留以下 blocker：Private Vulnerability Reporting 未启用；发布
资产缺少 SBOM、SHA-256 checksum 与 provenance/attestation；Docker、Desktop、Android、
RemoteBrowser 仍需 current-HEAD candidate receipts；首个版本、CHANGELOG 与 release set
尚未冻结；Android 已有 `apksigner` workflow，但仍缺 current-HEAD secrets、签名验证与安装
证据。开源治理文件、ruleset、
Dependabot/CodeQL/container scan、operator runbook 属 P1；toolchain pins、`.editorconfig`、
fuzz/performance/privacy policy 属 P2 advisory。本批只建立诚实 gate，不顺带声称这些缺口
已经关闭。

### 2.2 Deferred Workflows (推迟的工作流)

以下 workflow 不属于权威 release / CI 基线：

- `nightly.yml`: 不再要求每日构建；如未来重新需要，应先更新本章再新增 workflow。
- `speckit-sync-check.yml`: 不再作为 release / CI 的必需校验面；规格同步检查应由后续独立治理流程重新定义。

### 2.3 Security & Signing (安全签名)
*   **macOS**: 必须配置 `APPLE_SIGNING_IDENTITY` 和 `APPLE_PROVIDER_SHORT_NAME` 以通过 Gatekeeper。
*   **Update**: 使用 Tauri Updater 机制，公钥 (`pubkey.pem`) 硬编码在客户端，私钥仅在 CI Secret 中。
*   **Container**: 镜像使用 GitHub Actor 签名 (Keyless signing with Sigstore/Cosign optional).

## 3. Versioning (版本规范)

遵循 **Semantic Versioning 2.0.0** (`MAJOR.MINOR.PATCH`).

*   **MAJOR**: 做了不兼容的 API 修改 (e.g., Ledger 数据结构变更).
*   **MINOR**: 做了向下兼容的功能性新增 (e.g., 新增 UI 插件槽).
*   **PATCH**: 做了向下兼容的问题修正 (e.g., 修复渲染 Bug).

> [!IMPORTANT]
> **Data Compatibility**: 首个 stable 发布后，任何涉及 `Ledger` 或 Projection Workspace / Locator 存储结构的变更，**MUST** 提供迁移路径。首选 "Copy & Rebuild" 策略（见 03_storage/）；仅当无法重建时才提供增量迁移脚本，并在 Major 版本中发布。pre-1.0 阶段允许一次性不兼容重置，但必须更新 plan 与 release notes。
>
> 首个 stable 的持久化基线包含 `LEDGER_ENTRY_FORMAT_VERSION = 3` 与 `REDB_SCHEMA_VERSION = 3`，二者均使用 project-owned postcard codec payload。首个公开 WS wire epoch 为 `DEVEWSF4` / v1 lockstep；历史未发布 F2/F3 namespace 不进入兼容承诺，F4/v1 发布后只允许单调升级。v2 storage 仅保留显式只读导出后重建边界，不做来源推测或原地迁移。Projection Backup 不引入 ledger pack plaintext 格式；其 locator/transport 形态由 first-tag format matrix 钉住。pre-1.0 未发布开发期产生的无版本 ledger entry、旧 codec ledger entry 或旧 schema gate `.redb` 可以 fail-closed 并要求显式 reset / repair / migration，不进入 stable 兼容承诺。

## 4. Open Source License (开源协议)

采用 MIT License；再分发副本必须保留版权声明。

## 5. Docker Deployment (容器化部署)

支持通过 OCI 容器在服务器或 NAS 环境中运行 `deve-server`。

### 5.1 Run with Docker CLI
```bash
docker run -d \
  --name deve-server \
  -p 3001:3001 \
  -v $(pwd)/data:/data \
  -v $(pwd)/notes:/notes \
  -e DEVE_LEDGER_DIR=/data/ledger \
  -e AUTH_SECRET=<32-plus-byte-random-secret> \
  -e AUTH_USER=admin \
  -e AUTH_PASS='<argon2-phc-password-hash>' \
  ghcr.io/develata/deve-notebook:latest
```

### 5.2 Run with Docker Compose
```yaml
version: '3.8'
services:
  deve-server:
    image: ghcr.io/develata/deve-notebook:latest
    container_name: deve-server
    restart: always
    ports:
      - "3001:3001"
    volumes:
      - ./data:/data
      - ./notes:/notes
    environment:
      - DEVE_BIND_ADDR=0.0.0.0:3001
      - DEVE_LEDGER_DIR=/data/ledger
      - AUTH_SECRET=${AUTH_SECRET:?set AUTH_SECRET}
      - AUTH_USER=${AUTH_USER:-admin}
      - AUTH_PASS=${AUTH_PASS:?set AUTH_PASS}
```

容器部署 **MUST NOT** 假设 `/data/vault` 是全局投影根。每个本地 repo 的 projection base 必须先通过
`deve init --path <data-root> --repo <name> --projection-base <projection-base>` 或 `deve repo projection set --repo <selector> --base <projection-base>`
写入 host-local Projection Locator；实际 workspace root 为 `<projection-base>/<safe_repo_name>--<repo_id>/`。
例如 `--projection-base /notes --repo default` 对应 `/notes/default--<repo_id>/`。

### 5.3 Build Strategy
*   **Base Image**: `debian:bookworm-slim` 或 `gcr.io/distroless/cc-debian12` (Runtime).
*   **Builder**: `rust:1.92-bookworm` (Multi-stage build)，包含 Node.js、固定版本的 Cargo-installed tools（当前为 `trunk`）与 `wasm32-unknown-unknown` target。
*   **Optimization**: Docker 发布基线 **MUST** 使用 locked direct release build；依赖缓存层属于可选构建优化，只有在 locked CI 与 Docker smoke 通过后才可进入发布基线。
*   **Frontend Delivery**: runtime image 只交付单个嵌入前端静态资源的 `deve_cli` 二进制；正常 Docker 部署 **MUST NOT** 依赖 `/app/static` 或 `DEVE_STATIC_DIR`。嵌入或显式静态根的 `index.html` **MUST NOT** 包含 Trunk development live-reload 标记；显式 `DEVE_STATIC_DIR` 命中该类 index 时 fail-closed，嵌入式前端命中该类 index 时不得被报告或服务为 `embedded-frontend`，发布 smoke 不能只依赖 `/api/node/role` 的 `api-only`，还必须用浏览器入口证明 release frontend 可用。
*   **Local Smoke Diagnostics**: `scripts/smoke-docker-release.sh` **MUST** 支持 `DEVE_DOCKER_BIN` 以覆盖非默认 Docker CLI 路径，并在 Docker 缺失或不可达时输出 Docker binary/context 诊断。release 与 multiclient smoke 还必须支持显式 existing-image 模式；该模式要求 image 已存在、禁止重新 build，并使 release workflow 能证明运行与浏览器 smoke 覆盖的是即将发布的同一 image ID。

### 5.4 Runtime Observability {#runtime-observability}

公开 `/api/node/role` endpoint 是面向 smoke test 与运维的轻量 release/runtime shape
观测面。它 **MUST** 暴露 version、profile、delivery shape、environment、ports 与聚合 repo
health counts。degraded repo 的细节仍只属于 CLI/admin diagnostics；公开 endpoint 只能返回
聚合计数，以便运维发现 degraded startup，同时避免泄漏 repo name 或 corruption detail。

Web dashboard 通过 `SystemMetrics` 展示的 CPU / memory gauge 必须遵循
`22_reliability_observability#metrics-taxonomy` 定义的 runtime resource domain、cgroup
选择与完整回退规则；本章只拥有 release/runtime 观测入口，不重复定义 metric source 语义。

### 5.5 Docker P2P Mesh Smoke

发布前的本地 Docker mesh smoke **MAY** 使用独立 compose override 启动两个 `deve-server`
实例。该 smoke 必须使用隔离 volume、显式共享 `RepoId`、静态 peer 配置与 env token，
并验证：

- 两个服务端各自拥有独立 local ledger。
- A 的本地写入只进入 B 的 A-shadow。
- B 的 local branch 在显式 merge 前不被污染。
- 断线重连后重新 `SyncHello` 并按当前 vector 对齐。
- smoke 必须故障注入一个 source peer sequence 缺口，证明后续事实被阻塞、shadow/vector 不推进；恢复缺失事实后才能连续收敛。

该 smoke 只证明 server-to-server mesh runtime；不等价于 native release、store readiness、
公网 discovery 或 NAT traversal readiness。

## 6. Checklist for Release (发布清单)

发布前 (Pre-flight Check) 必须确认：

- [ ] 所有 CI 测试通过 (Green).
- [ ] `CHANGELOG.md` 已更新。
- [ ] 关键依赖 (Dependencies) 无高危审计漏洞 (`cargo audit`, `npm audit`).
- [ ] 非漏洞依赖 warning 均有 registry allowlist 理由或替换路线；首个正式 tag 前
      `tag_blocker=yes` 项已被 USER 决策、替换或重新归类。
- [ ] Remote Projection S3-compatible credential binding 遵循 ADR 0008 的长期 profile
      contract；CLI 显式 profile slice 可执行，未绑定 / locator-profile 不匹配 / Web
      profile UX 尚未接入的 custom endpoint 必须继续 fail-closed，且不得把默认
      `AWS_*` 环境凭证签给任意 custom host。
- [ ] Linux native Desktop first-tag TODO 已关闭，或首个正式 tag 的 release set
      已明确排除 Linux native Desktop artifacts，并将 GTK4/WebKitGTK 6-compatible
      Tauri/Wry route 或等价 maintained WebView route 记录为后续工作。
- [ ] 多平台 (Win/Mac/Linux) 冒烟测试通过；若 Linux native Desktop artifact 被
      首个正式 tag 排除，本项的 Linux native package/startup 部分不作为该 tag
      的 release blocker，但必须保留为后续 evidence gap。
