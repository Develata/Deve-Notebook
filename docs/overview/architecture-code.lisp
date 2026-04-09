;;; architecture-code.lisp
;;; Architecture view derived from actual source tree:
;;;   - apps/cli/src/commands/*.rs          (CLI entry points)
;;;   - apps/cli/src/server/router.rs       (HTTP/WS routes)
;;;   - apps/cli/src/server/handlers/*.rs   (Application-layer handlers)
;;;   - crates/core/src/*/mod.rs            (Core modules)
;;; Format: keyword-style s-expression.
;;; Generated: 2026-04-09 (manual baseline)
;;; Source of truth: code layer. Cross-check against architecture-doc.lisp.

(system :name deve-note
        :version "0.0.1"
        :layers (user-entry application module core))

;; =============================================================================
;; Layer 1 — User Entry
;; Source: apps/cli/src/commands/mod.rs (pub mod lines)
;; =============================================================================

(layer :id user-entry
       :order 1
       :description "CLI subcommands that ship today. Web palette/slash commands live in web callbacks.")

(user-entry :id cli.init       :label "deve init"       :kind cli-command :code "apps/cli/src/commands/init.rs"       :calls (app.init))
(user-entry :id cli.scan       :label "deve scan"       :kind cli-command :code "apps/cli/src/commands/scan.rs"       :calls (app.scan))
(user-entry :id cli.watch      :label "deve watch"      :kind cli-command :code "apps/cli/src/commands/watch.rs"      :calls (app.watch))
(user-entry :id cli.serve      :label "deve serve"      :kind cli-command :code "apps/cli/src/commands/serve.rs"      :calls (app.serve))
(user-entry :id cli.dump       :label "deve dump"       :kind cli-command :code "apps/cli/src/commands/dump.rs"       :calls (app.dump))
(user-entry :id cli.export     :label "deve export"     :kind cli-command :code "apps/cli/src/commands/export.rs"     :calls (app.export))
(user-entry :id cli.verify-p2p :label "deve verify-p2p" :kind cli-command :code "apps/cli/src/commands/verify_p2p.rs" :calls (app.verify-p2p))
(user-entry :id cli.seed       :label "deve seed"       :kind cli-command :code "apps/cli/src/commands/seed.rs"       :calls (app.seed))
(user-entry :id cli.node-check :label "deve node-check" :kind cli-command :code "apps/cli/src/commands/node_check.rs" :calls (app.admin.node-check))
(user-entry :id cli.recover    :label "deve recover"    :kind cli-command :code "apps/cli/src/commands/recover.rs"    :calls (app.recover))
(user-entry :id cli.repair     :label "deve repair"     :kind cli-command :code "apps/cli/src/commands/repair/"     :calls (app.repair))
(user-entry :id cli.live-proxy :label "deve live-proxy" :kind cli-command :code "apps/cli/src/commands/live_proxy.rs" :calls (app.live-proxy))

;; =============================================================================
;; Layer 2 — Application
;; Source: apps/cli/src/server/router.rs route() chain + apps/cli/src/commands/*
;; =============================================================================

(layer :id application
       :order 2
       :description "CLI subcommand implementations and Axum handlers registered in server/router.rs.")

;; CLI handlers
(application :id app.init        :label "commands::init::run"      :kind cli-handler :code "apps/cli/src/commands/init.rs"       :calls (core.ledger core.tree))
(application :id app.scan        :label "commands::scan::run"      :kind cli-handler :code "apps/cli/src/commands/scan.rs"       :calls (core.sync core.tree))
(application :id app.watch       :label "commands::watch::run"     :kind cli-handler :code "apps/cli/src/commands/watch.rs"      :calls (core.watcher core.sync))
(application :id app.serve       :label "commands::serve::run"     :kind cli-handler :code "apps/cli/src/commands/serve.rs"      :calls (core.protocol core.security))
(application :id app.dump        :label "commands::dump::run"      :kind cli-handler :code "apps/cli/src/commands/dump.rs"       :calls (core.ledger))
(application :id app.export      :label "commands::export::run"    :kind cli-handler :code "apps/cli/src/commands/export.rs"     :calls (core.ledger))
(application :id app.verify-p2p  :label "commands::verify_p2p"     :kind cli-handler :code "apps/cli/src/commands/verify_p2p.rs" :calls (core.protocol))
(application :id app.seed        :label "commands::seed::run"      :kind cli-handler :code "apps/cli/src/commands/seed.rs"       :calls (core.ledger))
(application :id app.recover     :label "commands::recover::run"   :kind cli-handler :code "apps/cli/src/commands/recover.rs"    :calls (core.ledger core.sync))
(application :id app.repair      :label "commands::repair::run"    :kind cli-handler :code "apps/cli/src/commands/repair/"    :calls (core.sync core.tree))
(application :id app.live-proxy  :label "commands::live_proxy"     :kind cli-handler :code "apps/cli/src/commands/live_proxy.rs" :calls (core.ledger))

;; WS route
(application :id app.ws-handler  :label "ws::ws_handler"  :kind ws-handler  :code "apps/cli/src/server/ws/" :route "/ws" :calls (core.protocol core.sync))

;; Source Control HTTP routes (from router.rs lines 37-69)
(application :id app.sc.pending        :label "source_control::http::pending"                 :kind http-handler :route "/api/sc/pending"        :calls (core.source_control))
(application :id app.sc.status         :label "source_control::http::status"                  :kind http-handler :route "/api/sc/status"         :calls (core.source_control))
(application :id app.sc.diff           :label "source_control::http::diff"                    :kind http-handler :route "/api/sc/diff"           :calls (core.source_control))
(application :id app.sc.commits        :label "source_control::http_commits::commit_history"  :kind http-handler :route "/api/sc/commits"        :calls (core.source_control core.ledger))
(application :id app.sc.commit-diff    :label "source_control::http_commits::commit_diff"     :kind http-handler :route "/api/sc/commit-diff"    :calls (core.source_control))
(application :id app.sc.stage          :label "source_control::http_mutations::stage"         :kind http-handler :route "/api/sc/stage-pending" :calls (core.source_control))
(application :id app.sc.unstage        :label "source_control::http_mutations::unstage"       :kind http-handler :route "/api/sc/unstage"        :calls (core.source_control))
(application :id app.sc.discard        :label "source_control::http_mutations::discard_pending" :kind http-handler :route "/api/sc/discard-pending" :calls (core.source_control))
(application :id app.sc.commit         :label "source_control::http_mutations::commit"        :kind http-handler :route "/api/sc/commit"         :calls (core.source_control core.ledger))

;; Repo HTTP routes
(application :id app.repo.list      :label "repo::http::list_docs"    :kind http-handler :route "/api/repo/docs" :calls (core.tree core.ledger))
(application :id app.repo.doc       :label "repo::http::doc_content"  :kind http-handler :route "/api/repo/doc"  :calls (core.tree core.ledger))

;; Auth HTTP routes
(application :id app.auth.login     :label "auth::handlers::login"   :kind http-handler :route "/api/auth/login"  :calls (core.security))
(application :id app.auth.logout    :label "auth::handlers::logout"  :kind http-handler :route "/api/auth/logout" :calls (core.security))
(application :id app.auth.me        :label "auth::handlers::me"      :kind http-handler :route "/api/auth/me"     :calls (core.security))

;; Admin HTTP routes
(application :id app.admin.dump       :label "admin::dump"       :kind http-handler :route "/api/admin/dump"       :calls (core.ledger))
(application :id app.admin.export     :label "admin::export"     :kind http-handler :route "/api/admin/export"     :calls (core.ledger))
(application :id app.admin.node-check :label "admin::node_check" :kind http-handler :route "/api/admin/node-check" :calls (core.tree))

;; Public
(application :id app.node.role      :label "node_role_http::role" :kind http-handler :route "/api/node/role" :calls (core.protocol))

;; =============================================================================
;; Layer 3 — Module (leaf engineering modules, actually existing sub-modules)
;; Source: crates/core/src/*/mod.rs + known sub-directories
;; =============================================================================

(layer :id module
       :order 3
       :description "Concrete leaf modules under crates/core/src/.")

;; sync sub-modules
(module :id core.watcher         :label "sync::watcher"       :parent core.sync           :code "crates/core/src/sync/watcher/")
(module :id core.drift_detect    :label "sync::drift_detect"  :parent core.sync           :code "crates/core/src/sync/drift_detect/")
(module :id core.materialize     :label "sync::materialize"   :parent core.sync           :code "crates/core/src/sync/materialize.rs")
(module :id core.rebuild         :label "sync::rebuild"       :parent core.sync           :code "crates/core/src/sync/rebuild.rs")
(module :id core.projection_plan :label "sync::projection_plan" :parent core.sync         :code "crates/core/src/sync/projection_plan.rs")
(module :id core.projection_io   :label "sync::projection_io" :parent core.sync           :code "crates/core/src/sync/projection_io.rs")

;; source_control sub-modules
(module :id core.pending_fs      :label "source_control::pending_fs" :parent core.source_control :code "crates/core/src/source_control/pending_fs.rs")
(module :id core.staging         :label "source_control::staging"    :parent core.source_control :code "crates/core/src/source_control/staging.rs")
(module :id core.sc_diff         :label "source_control::diff"       :parent core.source_control :code "crates/core/src/source_control/diff.rs")

;; ledger sub-modules
(module :id core.ledger.manager  :label "ledger::manager"    :parent core.ledger :code "crates/core/src/ledger/manager/")
(module :id core.ledger.schema   :label "ledger::schema"     :parent core.ledger :code "crates/core/src/ledger/schema.rs")
(module :id core.ledger.append   :label "ledger::append"     :parent core.ledger :code "crates/core/src/ledger/append.rs")

;; tree sub-modules
(module :id core.tree.structure  :label "tree::structure"    :parent core.tree :code "crates/core/src/tree/")

;; protocol sub-modules
(module :id core.protocol.ws     :label "protocol::ws"       :parent core.protocol :code "crates/core/src/protocol/")

;; security sub-modules
(module :id core.security.auth   :label "security::auth"     :parent core.security :code "crates/core/src/security/auth/")

;; plugin sub-modules
(module :id core.plugin.chat     :label "plugin::chat_stream" :parent core.plugin :code "crates/core/src/plugin/")

;; search sub-modules
(module :id core.search.index    :label "search::index" :parent core.search :code "crates/core/src/search/")

;; Extra modules present in code but less prominent in docs
(module :id core.context         :label "context"     :parent core.core-misc :code "crates/core/src/context/")
(module :id core.mcp             :label "mcp"         :parent core.core-misc :code "crates/core/src/mcp/")
(module :id core.skill           :label "skill"       :parent core.core-misc :code "crates/core/src/skill/")
(module :id core.utils           :label "utils"       :parent core.core-misc :code "crates/core/src/utils/")
(module :id core.vfs             :label "vfs"         :parent core.core-misc :code "crates/core/src/vfs.rs")
(module :id core.state           :label "state"       :parent core.core-misc :code "crates/core/src/state.rs")
(module :id core.models          :label "models"      :parent core.core-misc :code "crates/core/src/models.rs")
(module :id core.config          :label "config"      :parent core.core-misc :code "crates/core/src/config.rs")
(module :id core.error           :label "error"       :parent core.core-misc :code "crates/core/src/error.rs")

;; =============================================================================
;; Layer 4 — Core (top-level subsystems in crates/core/src/)
;; Source: actual sub-directories of crates/core/src/
;; =============================================================================

(layer :id core
       :order 4
       :description "Top-level directories under crates/core/src/. Each owns one authority domain.")

(core :id core.ledger         :label "ledger"         :kind authority   :code "crates/core/src/ledger/")
(core :id core.sync           :label "sync"           :kind runtime     :code "crates/core/src/sync/")
(core :id core.source_control :label "source_control" :kind workflow    :code "crates/core/src/source_control/")
(core :id core.tree           :label "tree"           :kind projection  :code "crates/core/src/tree/")
(core :id core.protocol       :label "protocol"       :kind runtime     :code "crates/core/src/protocol/")
(core :id core.security       :label "security"       :kind runtime     :code "crates/core/src/security/")
(core :id core.plugin         :label "plugin"         :kind peripheral  :code "crates/core/src/plugin/")
(core :id core.search         :label "search"         :kind peripheral  :code "crates/core/src/search/")
(core :id core.watcher        :label "watcher (root)" :kind runtime     :code "crates/core/src/watcher.rs")
(core :id core.core-misc      :label "core-misc"      :kind misc        :code "crates/core/src/ (context/mcp/skill/utils/vfs/state/models/config/error)")
