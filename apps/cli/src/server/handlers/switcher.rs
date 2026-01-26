use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::listing;
use crate::server::session::WsSession;
use deve_core::ledger::listing::RepoListing;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

/// 处理分支切换
pub async fn handle_switch_branch(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    peer_id: Option<String>,
) {
    tracing::info!("Handle SwitchBranch request: PeerID={:?}", peer_id);

    let mut final_branch = peer_id.clone();
    let mut force_repo_switch = None;

    // --- DEFENSIVE LOGIC START ---
    // Detect if the requested 'peer_id' is actually a Local Repository name.
    // This handles cases where the Frontend might mistakenly send SwitchBranch instead of SwitchRepo,
    // or passes a Repo Name as a Peer ID.
    if let Some(pid_str) = &peer_id {
        // 1. Check if this ID exists as a Shadow (Remote Peer)
        let shadows = state.repo.list_shadows_on_disk().unwrap_or_default();
        let is_valid_shadow = shadows.iter().any(|p| p.as_str() == pid_str);

        // 2. Check if this ID exists as a Local Repo
        let local_repos = state.repo.list_repos(None).unwrap_or_default();
        let is_local_repo = local_repos.contains(pid_str);

        if !is_valid_shadow && is_local_repo {
            tracing::warn!(
                "Suspicious SwitchBranch: '{}' is a Local Repo but not a Shadow. Correcting to Local Mode.",
                pid_str
            );
            // Correction: Switch to Local Branch (None) and set Active Repo to this name
            final_branch = None;
            force_repo_switch = Some(pid_str.clone());
        }
    }
    // --- DEFENSIVE LOGIC END ---

    // 1. 获取当前 Repo 的 URL (用于智能切换, if staying in same context style)
    // Only capture if we are NOT forcing a switch
    let current_repo_url = if force_repo_switch.is_none() {
        if let Some(current_repo) = &session.active_repo {
            state
                .repo
                .get_repo_url(session.active_branch.as_ref(), current_repo)
                .ok()
                .flatten()
        } else {
            None
        }
    } else {
        None
    };

    // 2. 切换 Session 分支状态
    session.switch_branch(final_branch.clone());
    tracing::info!(
        "Session ActiveBranch updated to: {:?}",
        session.active_branch
    );

    ch.unicast(ServerMessage::BranchSwitched {
        peer_id: final_branch.clone(),
        success: true,
    });

    // 3. 确定目标 Repo
    let target_repo = if let Some(forced) = force_repo_switch {
        Some(forced)
    } else {
        // 自动切Repo: 查找相同 URL 的 Repo，或者默认第一个
        // Important: check if branch exists to avoid creating empty folders via list_repos?
        // list_repos does not create folders. Safe.
        let repos = state
            .repo
            .list_repos(session.active_branch.as_ref())
            .unwrap_or_default();

        let mut best_match = None;

        // 策略 A: 尝试匹配 URL
        if let Some(url) = &current_repo_url {
            for repo_name in &repos {
                // get_repo_url might create shadow DB if branch is Some!
                // We should only call it if we are sure?
                // Actually, list_repos(Some) only returns repos that exist (as .redb files).
                // So get_repo_url shouldn't trigger creation of NEW dirs if repo exists.
                // But list_repos returns what's on disk.

                if let Ok(Some(r_url)) = state
                    .repo
                    .get_repo_url(session.active_branch.as_ref(), repo_name)
                {
                    if r_url == *url {
                        best_match = Some(repo_name.clone());
                        break;
                    }
                }
            }
        }

        // 策略 B: 默认第一个 (Alphabetical First)
        if best_match.is_none() {
            if let Some(first) = repos.first() {
                best_match = Some(first.clone());
            }
        }
        best_match
    };

    // 4. 执行 Repo 切换并锁定数据库
    if let Some(repo_name) = target_repo {
        tracing::info!("Auto-switching to repo: {}", repo_name);
        session.switch_repo(repo_name.clone());

        // 锁定数据库
        match state
            .repo
            .open_database(session.active_branch.as_ref(), &repo_name)
        {
            Ok(handle) => {
                session.set_active_db(handle);
                tracing::info!(
                    "Database locked: {} (readonly: {})",
                    repo_name,
                    session.is_readonly()
                );
            }
            Err(e) => {
                tracing::error!("Failed to lock database: {:?}", e);
                // 继续，但不设置 active_db
            }
        }

        ch.unicast(ServerMessage::RepoSwitched {
            name: repo_name.clone(),
            uuid: "".to_string(), // TODO: Fetch UUID
        });
    } else {
        // If no repo found (e.g. empty branch), clear active db
        session.active_db = None;
    }

    // 5. 刷新列表
    // Crucial: Use the current session state which uses `active_branch`
    // If active_branch is None (Local), this calls list_local_docs.
    // If active_branch is Some (Remote), this calls list_docs(Remote), which triggers ensure_shadow_db.
    // This is valid IF the branch actually exists (which we verified or corrected above).
    listing::handle_list_docs(
        state,
        ch,
        session.active_branch.as_ref(), // Updated branch
        session.active_repo.as_ref(),   // Updated repo
    )
    .await;
    listing::handle_list_repos(state, ch, session.active_branch.as_ref()).await;
}

/// 处理仓库切换
pub async fn handle_switch_repo(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    name: String,
) {
    tracing::info!(
        "Handle SwitchRepo request: Name='{}', CurrentBranch={:?}",
        name,
        session.active_branch
    );

    // 1. 验证目标 Repo 是否存在于 CURRENT Branch
    let branch = session.active_branch.clone();
    let repos = state.repo.list_repos(branch.as_ref()).unwrap_or_default();

    if repos.contains(&name) {
        // 2. 切换 Session Repo 状态
        session.switch_repo(name.clone());
        tracing::info!("Client switched to repo: {} (Branch: {:?})", name, branch);

        // 3. 锁定数据库
        match state.repo.open_database(branch.as_ref(), &name) {
            Ok(handle) => {
                session.set_active_db(handle);
                tracing::info!(
                    "Database locked: {} (readonly: {})",
                    name,
                    session.is_readonly()
                );
            }
            Err(e) => {
                tracing::error!("Failed to lock database: {:?}", e);
                ch.send_error(format!("Failed to lock database: {}", e));
                return;
            }
        }

        ch.unicast(ServerMessage::RepoSwitched {
            name: name.clone(),
            uuid: "".to_string(), // TODO: Fetch UUID
        });

        // 4. 刷新文档列表 (使用新的 Repo 上下文)
        listing::handle_list_docs(
            state,
            ch,
            session.active_branch.as_ref(),
            session.active_repo.as_ref(),
        )
        .await;
    } else {
        tracing::warn!(
            "Repo switch failed: '{}' not found in branch {:?}. Available: {:?}",
            name,
            branch,
            repos
        );
        ch.send_error(format!("Repository not found: {}", name));
    }
}
