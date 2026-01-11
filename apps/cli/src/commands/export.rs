use std::path::PathBuf;
use std::fs::File;
use std::io::{Write, BufWriter};
use deve_core::ledger::Ledger;
use deve_core::models::{DocId, LedgerEntry};
use anyhow::Result;
use serde::Serialize;

#[derive(Serialize)]
struct ExportEntry {
    doc_id: DocId,
    path: String,
    ops: Vec<LedgerEntry>,
}

pub fn run(ledger_path: &PathBuf, output: Option<String>, snapshot_depth: usize) -> Result<()> {
    let ledger = Ledger::init(ledger_path, snapshot_depth)?;
    let docs = ledger.list_docs()?;
    
    let mut writer: Box<dyn Write> = if let Some(path) = output {
        let file = File::create(path)?;
        Box::new(BufWriter::new(file))
    } else {
        Box::new(std::io::stdout())
    };
    
    for (doc_id, path) in docs {
        let ops_with_seq = ledger.get_ops(doc_id)?;
        let ops: Vec<LedgerEntry> = ops_with_seq.into_iter().map(|(_, op)| op).collect();
        
        let entry = ExportEntry {
            doc_id,
            path,
            ops,
        };
        
        let json = serde_json::to_string(&entry)?;
        writeln!(writer, "{}", json)?;
    }
    
    Ok(())
}
