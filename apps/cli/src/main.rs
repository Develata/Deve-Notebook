use clap::{Parser, Subcommand};
use deve_core::ledger::Ledger;
use deve_core::vfs::Vfs;
use std::path::PathBuf;

mod server;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize a new Deve-Note vault
    Init {
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
    },
    /// Scan and index the vault
    Scan,
    /// Watch the vault for changes
    Watch,
    /// Dump ops for a file
    Dump {
        #[arg(short, long)]
        path: String,
    },
    /// Start the backend server
    Serve {
        #[arg(short, long, default_value_t = 3001)]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Default paths
    let ledger_path = PathBuf::from("deve.db");
    let vault_path = PathBuf::from("vault");

    match args.command {
        Some(Commands::Init { path }) => {
            println!("Initializing ledger at {:?}...", ledger_path);
            let _ = Ledger::init(&ledger_path)?;
            std::fs::create_dir_all(&vault_path)?;
            println!("Initialization complete.");
        }
        Some(Commands::Scan) => {
            let ledger = Ledger::init(&ledger_path)?;
            let vfs = Vfs::new(&vault_path);
            println!("Scanning vault at {:?}...", vault_path);
            let count = vfs.scan(&ledger)?;
            println!("Scanned. Registered {} new documents.", count);
        }
        Some(Commands::Watch) => {
            let ledger = Arc::new(Ledger::init(&ledger_path)?);
            let vfs = Vfs::new(&vault_path);
            let watcher = deve_core::watcher::Watcher::new(ledger, vfs);
            println!("Starting watcher on {:?}... Press Ctrl+C to stop.", vault_path);
            watcher.watch()?;
        }
        Some(Commands::Dump { path }) => {
            let ledger = Ledger::init(&ledger_path)?;
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
        }
        Some(Commands::Serve { port }) => {
            let ledger = Ledger::init(&ledger_path)?;
            server::start_server(ledger, vault_path, port).await?;
        }
        None => {
            println!("Please provide a subcommand. Try --help.");
        }
    }

    Ok(())
}
