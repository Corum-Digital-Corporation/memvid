//! Memvid CLI - Command-line interface for memvid-core
//!
//! Provides create, put, search, stats, and timeline operations.

use clap::{Parser, Subcommand};
use memvid_core::{Memvid, PutOptions, Result, SearchRequest, TimelineQuery};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "memvid")]
#[command(about = "Single-file memory layer for AI agents", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new memory file
    Create {
        /// Path to the .mv2 file
        path: PathBuf,
    },

    /// Add text content to a memory
    Put {
        /// Path to the .mv2 file
        path: PathBuf,
        /// Text content to add
        content: String,
        /// Optional title for the content
        #[arg(long)]
        title: Option<String>,
        /// Optional URI identifier
        #[arg(long)]
        uri: Option<String>,
    },

    /// Ingest a file into the memory
    Ingest {
        /// Path to the .mv2 file
        path: PathBuf,
        /// Path to the file to ingest
        file: PathBuf,
        /// Optional title (defaults to filename)
        #[arg(long)]
        title: Option<String>,
    },

    /// Search the memory
    Search {
        /// Path to the .mv2 file
        path: PathBuf,
        /// Search query
        query: String,
        /// Number of results to return
        #[arg(long, default_value = "10")]
        top_k: usize,
    },

    /// Show memory statistics
    Stats {
        /// Path to the .mv2 file
        path: PathBuf,
    },

    /// Show timeline of entries
    Timeline {
        /// Path to the .mv2 file
        path: PathBuf,
        /// Maximum number of entries
        #[arg(long, default_value = "20")]
        limit: usize,
    },

    /// Verify file integrity
    Verify {
        /// Path to the .mv2 file
        path: PathBuf,
        /// Perform deep verification
        #[arg(long)]
        deep: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create { path } => {
            let mem = Memvid::create(&path)?;
            drop(mem);
            println!("{{\"status\": \"ok\", \"path\": \"{}\"}}", path.display());
        }

        Commands::Put {
            path,
            content,
            title,
            uri,
        } => {
            let mut mem = Memvid::open(&path)?;
            let mut options = PutOptions::builder();
            if let Some(t) = &title {
                options = options.title(t);
            }
            if let Some(u) = &uri {
                options = options.uri(u);
            }
            let seq = mem.put_bytes_with_options(content.as_bytes(), options.build())?;
            mem.commit()?;
            println!("{{\"status\": \"ok\", \"sequence\": {}}}", seq);
        }

        Commands::Ingest { path, file, title } => {
            let mut mem = Memvid::open(&path)?;
            let content = fs::read(&file)?;
            let file_title = title.unwrap_or_else(|| {
                file.file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "untitled".to_string())
            });
            let options = PutOptions::builder()
                .title(&file_title)
                .uri(&format!("file://{}", file.display()))
                .build();
            let seq = mem.put_bytes_with_options(&content, options)?;
            mem.commit()?;
            println!(
                "{{\"status\": \"ok\", \"sequence\": {}, \"title\": \"{}\"}}",
                seq, file_title
            );
        }

        Commands::Search { path, query, top_k } => {
            let mut mem = Memvid::open(&path)?;
            let request = SearchRequest {
                query: query.clone(),
                top_k,
                snippet_chars: 200,
                uri: None,
                scope: None,
                cursor: None,
                #[cfg(feature = "temporal_track")]
                temporal: None,
                as_of_frame: None,
                as_of_ts: None,
                no_sketch: false,
            };
            let response = mem.search(request)?;
            let hits: Vec<serde_json::Value> = response
                .hits
                .iter()
                .map(|h| {
                    serde_json::json!({
                        "frame_id": h.frame_id,
                        "title": h.title,
                        "score": h.score,
                        "text": h.text,
                        "uri": h.uri
                    })
                })
                .collect();
            println!(
                "{}",
                serde_json::json!({
                    "query": query,
                    "total_hits": response.total_hits,
                    "elapsed_ms": response.elapsed_ms,
                    "hits": hits
                })
            );
        }

        Commands::Stats { path } => {
            let mem = Memvid::open(&path)?;
            let stats = mem.stats()?;
            println!(
                "{}",
                serde_json::json!({
                    "frame_count": stats.frame_count,
                    "has_lex_index": stats.has_lex_index,
                    "has_vec_index": stats.has_vec_index,
                    "has_time_index": stats.has_time_index
                })
            );
        }

        Commands::Timeline { path, limit } => {
            let mut mem = Memvid::open(&path)?;
            let mut query = TimelineQuery::default();
            query.limit = std::num::NonZeroU64::new(limit as u64);
            let entries = mem.timeline(query)?;
            let items: Vec<serde_json::Value> = entries
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "frame_id": e.frame_id,
                        "uri": e.uri,
                        "preview": e.preview
                    })
                })
                .collect();
            println!("{}", serde_json::json!({ "entries": items }));
        }

        Commands::Verify { path, deep } => {
            let report = Memvid::verify(&path, deep)?;
            println!(
                "{}",
                serde_json::json!({
                    "status": format!("{:?}", report.overall_status),
                    "deep": deep
                })
            );
        }
    }

    Ok(())
}
