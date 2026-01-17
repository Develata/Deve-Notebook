use std::path::PathBuf;
use anyhow::{Result, Context};
use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::{PeerId, LedgerEntry, RepoType};

pub fn run(ledger_dir: &PathBuf, target_peer: String, snapshot_depth: usize) -> Result<()> {
    tracing::info!("Starting Seed Command...");
    tracing::info!("Ledger Dir: {:?}", ledger_dir);
    tracing::info!("Target Peer: {}", target_peer);

    let repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    let peer_id = PeerId::new(&target_peer);
    
    // Default Repo ID (Nil)
    let repo_id = uuid::Uuid::nil();

    // 1. List all local docs
    let docs = repo.list_docs(&RepoType::Local(uuid::Uuid::nil()))?;
    tracing::info!("Found {} local documents to seed.", docs.len());

    let mut total_ops = 0;

    for (doc_id, path) in docs {
        tracing::info!("Seeding doc: {} ({})", path, doc_id);
        
        // 2. Get local ops
        let ops = repo.get_local_ops(doc_id)?;
        
        // 3. Append to remote shadow
        for (_, entry) in ops {
            repo.append_remote_op(&peer_id, &repo_id, &entry)?;
            total_ops += 1;
        }
    }

    tracing::info!("✅ Seed completed successfully.");
    tracing::info!("Total Ops Copied: {}", total_ops);
    tracing::info!("Shadow Repo: remotes/{}/{}.redb", target_peer, repo_id);

    Ok(())
}
