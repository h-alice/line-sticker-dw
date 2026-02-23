use async_compat::Compat;
use clap::Parser;
use futures::future::join_all;
use std::path::PathBuf;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

mod sticker;

/// Download LINE sticker sets or emoji packs to your local machine.
#[derive(Parser, Debug)]
#[command(
    name = "line-sticker-dw",
    version,
    about = "Download LINE sticker sets or emoji packs",
    long_about = None
)]
struct Args {
    /// ID of the sticker set or emoji pack to download
    set_id: String,

    /// Optional output folder (defaults to <set_id>)
    base_folder: Option<PathBuf>,

    /// Download an emoji pack instead of a normal sticker set
    #[arg(long)]
    emoji: bool,

    /// Set log level to debug
    #[arg(short, long)]
    verbose: bool,
}

/// Fetches all stickers from a set and downloads them to a folder.
async fn download_set(
    id: &str,
    base_folder: Option<PathBuf>,
    emoji_mode: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let base_path = base_folder.unwrap_or_else(|| PathBuf::from(id));

    info!("Downloading sticker set {} to {}", id, base_path.display());

    // Get all sticker entries
    let stickers = sticker::fetch_stickers(id, emoji_mode).await?;
    info!(
        "Found {} stickers for set {}. Starting download...",
        stickers.len(),
        id
    );

    // Download all stickers to the folder
    // NOTE: expected in parallel, need investigation (smol vs tokio)
    let download_futures = stickers
        .iter()
        .map(|s| sticker::download_sticker_image(s, Some(base_path.clone())));

    let results = join_all(download_futures).await;

    // Check for any errors during download
    let mut success_count = 0;
    for result in results {
        match result {
            Ok(path) => {
                success_count += 1;
                debug_assert!(path.exists());
            }
            Err(e) => error!("Error downloading sticker: {}", e),
        }
    }

    info!(
        "Downloaded {}/{} stickers to {}/",
        success_count,
        stickers.len(),
        base_path.display()
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Log filter strategies
    let filter = if args.verbose {
        EnvFilter::new("line_sticker_dw=debug")
    } else {
        EnvFilter::new("line_sticker_dw=info")
    };

    tracing_subscriber::fmt().with_env_filter(filter).init();

    // Run the async workflow
    smol::block_on(Compat::new(async {
        if let Err(e) = download_set(&args.set_id, args.base_folder, args.emoji).await {
            error!("Failed to download sticker set: {}", e);
            std::process::exit(1);
        }
        Ok(())
    }))
}
