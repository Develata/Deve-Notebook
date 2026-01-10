use std::path::PathBuf;
use deve_core::ledger::Ledger;

pub fn run(ledger_path: &PathBuf, path: String) -> anyhow::Result<()> {
    let ledger = Ledger::init(ledger_path)?;
    if let Some(doc_id) = ledger.get_docid(&path)? {
        println!("DocId: {}", doc_id);
        let ops = ledger.get_ops(doc_id)?;
        println!("Found {} ops:", ops.len());
        for (i, (seq, entry)) in ops.iter().enumerate() {
            println!("[{}] Seq:{} {} {:?}", i, seq, entry.timestamp, entry.op);
        }
        
        let ops_vec: Vec<deve_core::models::LedgerEntry> = ops.iter().map(|(_, e)| e.clone()).collect();
        let content = deve_core::state::reconstruct_content(&ops_vec);
        println!("\nReconstructed Content:\n---\n{}\n---", content);
    } else {
        println!("Path not found in Ledger.");
    }
    Ok(())
}
