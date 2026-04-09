;;; architecture-doc.lisp
;;; Architecture view derived from docs/plan/*, docs/features/*, docs/acceptance-cases/*.
;;; Format: keyword-style s-expression.
;;; Generated: 2026-04-09 (manual baseline)
;;; Source of truth: documentation layer. Cross-check against architecture-code.lisp.

(system :name deve-note
        :version "0.0.1"
        :layers (user-entry application module core))

;; =============================================================================
;; Layer 1 — User Entry
;; Source: docs/plan/12_commands.md, docs/features/12_commands.md
;; =============================================================================

(layer :id user-entry
       :order 1
       :description "All user-facing command surfaces: CLI + Command Palette + Chat slash.")

(user-entry :id cli.init       :label "deve init"       :kind cli-command :chapter 12_commands    :calls (app.init))
(user-entry :id cli.scan       :label "deve scan"       :kind cli-command :chapter 12_commands    :calls (app.scan))
(user-entry :id cli.watch      :label "deve watch"      :kind cli-command :chapter 12_commands    :calls (app.watch))
(user-entry :id cli.serve      :label "deve serve"      :kind cli-command :chapter 12_commands    :calls (app.serve))
(user-entry :id cli.dump       :label "deve dump"       :kind cli-command :chapter 12_commands    :calls (app.dump))
(user-entry :id cli.export     :label "deve export"    :kind cli-command :chapter 12_commands    :calls (app.export))
(user-entry :id cli.verify-p2p :label "deve verify-p2p" :kind cli-command :chapter 12_commands    :calls (app.verify-p2p))
(user-entry :id cli.seed       :label "deve seed"       :kind cli-command :chapter 12_commands    :calls (app.seed))

(user-entry :id palette.cmdp      :label "Cmd+Shift+P / Ctrl+Shift+P"  :kind shortcut          :chapter 12_commands :calls (app.command-palette))
(user-entry :id palette.quickopen :label "Cmd+P / Ctrl+P"              :kind shortcut          :chapter 12_commands :calls (app.quick-open))
(user-entry :id palette.branch    :label "Cmd+Shift+K / Ctrl+Shift+K"  :kind shortcut          :chapter 12_commands :calls (app.branch-switch))

(user-entry :id palette.git.sync    :label "Git: Sync"    :kind palette-command :chapter 12_commands :calls (app.sc.sync))
(user-entry :id palette.git.commit  :label "Git: Commit"  :kind palette-command :chapter 12_commands :calls (app.sc.commit))
(user-entry :id palette.git.push    :label "Git: Push"    :kind palette-command :chapter 12_commands :calls (app.sc.push))

(user-entry :id palette.p2p.switch     :label "P2P: Switch to Peer"      :kind palette-command :chapter 12_commands :calls (app.p2p.switch))
(user-entry :id palette.p2p.establish  :label "P2P: Establish Branch"    :kind palette-command :chapter 12_commands :calls (app.p2p.establish))
(user-entry :id palette.p2p.merge      :label "P2P: Merge Peer"          :kind palette-command :chapter 12_commands :calls (app.p2p.merge))

(user-entry :id palette.ai.chat     :label "AI: Open Chat"              :kind palette-command :chapter 12_commands :calls (app.ai.chat))
(user-entry :id palette.ai.retry    :label "AI: Retry Last Request"     :kind palette-command :chapter 12_commands :calls (app.ai.retry))
(user-entry :id palette.ai.backend  :label "AI: Switch Backend"         :kind palette-command :chapter 12_commands :calls (app.ai.backend))
(user-entry :id palette.ai.plan     :label "AI: Switch to PLAN Mode"    :kind palette-command :chapter 12_commands :calls (app.ai.mode))
(user-entry :id palette.ai.build    :label "AI: Switch to BUILD Mode"   :kind palette-command :chapter 12_commands :calls (app.ai.mode))

(user-entry :id slash.plan    :label "/plan"    :kind chat-slash :chapter 12_commands :calls (app.ai.mode))
(user-entry :id slash.build   :label "/build"   :kind chat-slash :chapter 12_commands :calls (app.ai.mode))
(user-entry :id slash.agents  :label "/agents"  :kind chat-slash :chapter 12_commands :calls (app.ai.mode))

;; =============================================================================
;; Layer 2 — Application (HTTP / WS handlers, CLI command glue)
;; Source: Implied by docs/plan/05_network, 06_repository, 07_diff_logic, 09_auth, 16_web_thin_client_ledger
;; =============================================================================

(layer :id application
       :order 2
       :description "HTTP/WS handlers and CLI command glue. Route user intent into core modules.")

(application :id app.init       :label "cli::init"       :kind cli-handler  :chapter 02_positioning  :calls (core.ledger core.tree))
(application :id app.scan       :label "cli::scan"       :kind cli-handler  :chapter 04_storage      :calls (core.sync core.tree))
(application :id app.watch      :label "cli::watch"      :kind cli-handler  :chapter 04_storage      :calls (core.watcher core.sync))
(application :id app.serve      :label "cli::serve"      :kind cli-handler  :chapter 05_network      :calls (core.protocol core.security))
(application :id app.dump       :label "cli::dump"       :kind cli-handler  :chapter 04_storage      :calls (core.ledger))
(application :id app.export     :label "cli::export"     :kind cli-handler  :chapter 04_storage#backup-export :calls (core.ledger))
(application :id app.verify-p2p :label "cli::verify-p2p" :kind cli-handler  :chapter 05_network      :calls (core.protocol))
(application :id app.seed       :label "cli::seed"       :kind cli-handler  :chapter 04_storage      :calls (core.ledger))

(application :id app.ws-handler :label "server::ws"   :kind ws-handler   :chapter 16_web_thin_client_ledger :calls (core.protocol core.sync))
(application :id app.sc.pending :label "/api/sc/pending"  :kind http-handler :chapter 07_diff_logic :calls (core.source_control))
(application :id app.sc.status  :label "/api/sc/status"   :kind http-handler :chapter 07_diff_logic :calls (core.source_control))
(application :id app.sc.diff    :label "/api/sc/diff"     :kind http-handler :chapter 07_diff_logic :calls (core.source_control))
(application :id app.sc.stage   :label "/api/sc/stage"    :kind http-handler :chapter 07_diff_logic :calls (core.source_control))
(application :id app.sc.commit  :label "/api/sc/commit"   :kind http-handler :chapter 07_diff_logic :calls (core.source_control core.ledger))
(application :id app.sc.push    :label "/api/sc/push"     :kind http-handler :chapter 07_diff_logic :calls (core.source_control core.protocol))
(application :id app.sc.sync    :label "/api/sc/sync"     :kind http-handler :chapter 05_network    :calls (core.sync core.protocol))

(application :id app.repo.list      :label "/api/repo/docs"  :kind http-handler :chapter 06_repository :calls (core.tree core.ledger))
(application :id app.repo.doc       :label "/api/repo/doc"   :kind http-handler :chapter 06_repository :calls (core.tree core.ledger))

(application :id app.auth.login     :label "/api/auth/login"  :kind http-handler :chapter 09_auth#security-headers :calls (core.security))
(application :id app.auth.logout    :label "/api/auth/logout" :kind http-handler :chapter 09_auth#security-headers :calls (core.security))
(application :id app.auth.me        :label "/api/auth/me"     :kind http-handler :chapter 09_auth :calls (core.security))

(application :id app.admin.dump       :label "/api/admin/dump"       :kind http-handler :chapter 04_storage :calls (core.ledger))
(application :id app.admin.export     :label "/api/admin/export"     :kind http-handler :chapter 04_storage#backup-export :calls (core.ledger))
(application :id app.admin.node-check :label "/api/admin/node-check" :kind http-handler :chapter 04_storage :calls (core.tree))

(application :id app.command-palette  :label "command-palette"  :kind web-callback :chapter 08_ui_design :calls (app.sc.commit app.sc.push app.ai.chat))
(application :id app.quick-open       :label "quick-open"       :kind web-callback :chapter 08_ui_design :calls (app.repo.list))
(application :id app.branch-switch    :label "branch-switch"    :kind web-callback :chapter 06_repository :calls (core.tree))

(application :id app.p2p.switch    :label "p2p::switch"    :kind web-callback :chapter 05_network :calls (core.protocol core.sync))
(application :id app.p2p.establish :label "p2p::establish" :kind web-callback :chapter 05_network :calls (core.protocol core.tree))
(application :id app.p2p.merge     :label "p2p::merge"     :kind web-callback :chapter 05_network :calls (core.protocol core.source_control))

(application :id app.ai.chat    :label "ai::chat"    :kind web-callback :chapter 10_ai_agent :calls (core.plugin))
(application :id app.ai.retry   :label "ai::retry"   :kind web-callback :chapter 10_ai_agent :calls (core.plugin))
(application :id app.ai.backend :label "ai::backend" :kind web-callback :chapter 10_ai_agent :calls (core.plugin))
(application :id app.ai.mode    :label "ai::mode"    :kind web-callback :chapter 10_ai_agent :calls (core.plugin))

;; =============================================================================
;; Layer 3 — Module (leaf engineering modules)
;; Source: docs/plan/ Primary Code Areas fields
;; =============================================================================

(layer :id module
       :order 3
       :description "Finest-grained engineering modules — each owns a state machine or contract.")

(module :id core.watcher         :label "sync::watcher"       :chapter 04_storage#watcher-contract    :parent core.sync           :code-area "crates/core/src/sync/watcher/")
(module :id core.drift_detect    :label "sync::drift_detect"  :chapter 04_storage#projection-contract :parent core.sync           :code-area "crates/core/src/sync/drift_detect/")
(module :id core.materialize     :label "sync::materialize"   :chapter 04_storage                     :parent core.sync           :code-area "crates/core/src/sync/materialize.rs")
(module :id core.rebuild         :label "sync::rebuild"       :chapter 04_storage                     :parent core.sync           :code-area "crates/core/src/sync/rebuild.rs")
(module :id core.projection_plan :label "sync::projection_plan" :chapter 04_storage#projection-contract :parent core.sync         :code-area "crates/core/src/sync/projection_plan.rs")

(module :id core.pending_fs      :label "source_control::pending_fs" :chapter 04_storage#watcher-contract :parent core.source_control :code-area "crates/core/src/source_control/pending_fs.rs")
(module :id core.staging         :label "source_control::staging"    :chapter 07_diff_logic            :parent core.source_control :code-area "crates/core/src/source_control/staging.rs")
(module :id core.sc_diff         :label "source_control::diff"       :chapter 07_diff_logic            :parent core.source_control :code-area "crates/core/src/source_control/diff.rs")

(module :id core.ledger.manager  :label "ledger::manager"    :chapter 04_storage                     :parent core.ledger :code-area "crates/core/src/ledger/manager/")
(module :id core.ledger.schema   :label "ledger::schema"     :chapter 04_storage                     :parent core.ledger :code-area "crates/core/src/ledger/schema.rs")
(module :id core.ledger.append   :label "ledger::append"     :chapter 04_storage                     :parent core.ledger :code-area "crates/core/src/ledger/append.rs")

(module :id core.tree.structure  :label "tree::structure"    :chapter 06_repository :parent core.tree :code-area "crates/core/src/tree/structure/")

(module :id core.protocol.ws     :label "protocol::ws"       :chapter 05_network    :parent core.protocol :code-area "crates/core/src/protocol/")
(module :id core.protocol.handshake :label "protocol::handshake" :chapter 05_network :parent core.protocol :code-area "crates/core/src/protocol/handshake.rs")

(module :id core.security.auth   :label "security::auth"     :chapter 09_auth#security-headers :parent core.security :code-area "crates/core/src/security/auth/")
(module :id core.security.jwt    :label "security::jwt"      :chapter 09_auth :parent core.security :code-area "crates/core/src/security/auth/jwt.rs")

(module :id core.plugin.chat     :label "plugin::chat_stream" :chapter 10_ai_agent :parent core.plugin :code-area "crates/core/src/plugin/")

(module :id core.search.index    :label "search::index" :chapter 03_rendering :parent core.search :code-area "crates/core/src/search/")

;; =============================================================================
;; Layer 4 — Core (top-level subsystems)
;; Source: docs/plan/04_storage + 05_network + 06_repository + 07_diff_logic + 09_auth + 10_ai_agent
;; =============================================================================

(layer :id core
       :order 4
       :description "Top-level subsystems under crates/core/src/*. Each owns one authority domain.")

(core :id core.ledger         :label "Ledger"         :chapter 04_storage             :kind authority :code-area "crates/core/src/ledger/")
(core :id core.sync           :label "Sync"           :chapter 04_storage             :kind runtime   :code-area "crates/core/src/sync/")
(core :id core.source_control :label "Source Control" :chapter 07_diff_logic          :kind workflow  :code-area "crates/core/src/source_control/")
(core :id core.tree           :label "Tree"           :chapter 06_repository          :kind projection :code-area "crates/core/src/tree/")
(core :id core.protocol       :label "Protocol"       :chapter 05_network             :kind runtime   :code-area "crates/core/src/protocol/")
(core :id core.security       :label "Security"      :chapter 09_auth                :kind runtime   :code-area "crates/core/src/security/")
(core :id core.plugin         :label "Plugin"        :chapter 10_ai_agent            :kind peripheral :code-area "crates/core/src/plugin/")
(core :id core.search         :label "Search"        :chapter 03_rendering           :kind peripheral :code-area "crates/core/src/search/")
(core :id core.watcher        :label "Watcher (root)" :chapter 04_storage#watcher-contract :kind runtime :code-area "crates/core/src/watcher.rs")
