use anyhow::Result;
use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::{PeerId, LedgerEntry, RepoType, Op};
use std::path::PathBuf;
use uuid::Uuid;

fn main() -> Result<()> {
    let ledger_dir = PathBuf::from("ledger");
    // Ensure we can open the repo
    let repo = RepoManager::init(&ledger_dir, 1, None, None)?;
    // let default_repo_id = Uuid::nil(); // Unused

    // Inject into Peer A
    inject_conflict(&repo, "peera", "test_A.md", " [Remote A Change]")?;
    
    // Inject into Peer B
    inject_conflict(&repo, "peerb", "test_B.md", " [Remote B Change]")?;
    
    Ok(())
}

fn inject_conflict(repo: &RepoManager, peer_name: &str, filename: &str, content: &str) -> Result<()> {
    let peer_id = PeerId::new(peer_name);
    let repo_id = Uuid::nil();
    
    println!("Scanning local docs to find {}...", filename);
    
    // Use Local repo to find the DocId since Shadow Repos don't store path metadata
    let local_docs = repo.list_docs(&RepoType::Local(repo_id))?;
    
    let mut found = false;
    for (doc_id, path) in local_docs {
        // Match filename (handling potential path separators)
        if path.ends_with(filename) {
            println!("Found {} ({}), injecting conflict into {}...", path, doc_id, peer_name);
            found = true;
            
            // Create Op
            let entry = LedgerEntry {
                doc_id,
                op: Op::Insert {
                    pos: 0,
                    content: format!("CONFLICT_START{}CONFLICT_END\n", content),
                },
                timestamp: chrono::Utc::now().timestamp_millis(),
                peer_id: peer_id.clone(),
                seq: 99999, // High sequence number to simulate "latest"
            };
            
            repo.append_remote_op(&peer_id, &repo_id, &entry)?;
            println!("Success: Injected op into {}", peer_name);
        }
    }
    
    if !found {
        println!("Warning: Could not find document ending with {}", filename);
    }
    
    Ok(())
}
