use super::persist_guard::PersistGuard;
use super::rebuild;
use crate::ledger::RepoManager;
use crate::utils::path::to_forward_slash;
use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;
use walkdir::WalkDir;

/// 将指定本地 repo 的文档视图投影到 Vault。
///
/// Invariants:
/// - Vault 中的 `.md` 集合与目标 repo 的 Doc 集合一致。
/// - Projection 期间产生的自写回 FS 事件必须被抑制。
pub(super) fn materialize_local_repo(
    repo: &RepoManager,
    vault_root: &Path,
    guard: &PersistGuard,
    repo_name: &str,
) -> Result<()> {
    let docs = repo.list_local_docs(Some(repo_name))?;
    let desired: HashSet<String> = docs.iter().map(|(_, path)| path.clone()).collect();

    for (doc_id, path) in docs {
        let file_path = vault_root.join(&path);
        let rebuilt = rebuild::rebuild_local_doc_in_repo(repo, repo_name, doc_id)?;
        if std::fs::read_to_string(&file_path).unwrap_or_default() == rebuilt.content {
            continue;
        }
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        guard.record(&path, &rebuilt.content);
        std::fs::write(&file_path, rebuilt.content)?;
    }

    for entry in WalkDir::new(vault_root).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Ok(rel) = path.strip_prefix(vault_root) else {
            continue;
        };
        let rel_path = to_forward_slash(&rel.to_string_lossy());
        if rel_path.starts_with(".git")
            || rel_path.starts_with(".deve")
            || rel_path.starts_with(".notegit")
            || !rel_path.ends_with(".md")
            || desired.contains(&rel_path)
        {
            continue;
        }
        guard.record_delete(&rel_path);
        std::fs::remove_file(path)?;
    }

    Ok(())
}
