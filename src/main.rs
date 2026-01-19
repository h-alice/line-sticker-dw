use async_compat::Compat;
use futures::future::join_all;
use std::path::PathBuf;
use tracing::{Level, error, info};
use tracing_subscriber::FmtSubscriber;

mod sticker;

// A default help message.
const HELP: &str = "\
Usage: line-sticker-dw <set_id> [base_folder] [-v|--verbose]

Arguments:
  <set_id>       The ID of the sticker set to download
  [base_folder]  Optional base folder to save stickers (defaults to <set_id>)

Options:
  -v, --verbose  Set log level to debug
  -h, --help     Print help
";

/// Fetches all stickers from a set and downloads them to a folder.
async fn download_set(
    id: u64,
    base_folder: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let base_path = base_folder.unwrap_or_else(|| PathBuf::from(id.to_string()));

    info!("Downloading sticker set {} to {}", id, base_path.display());

    // Get all sticker entries
    let stickers = sticker::fetch_stickers(id).await?;
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
    let mut args = pico_args::Arguments::from_env();

    // Print help message
    if args.contains(["-h", "--help"]) {
        print!("{}", HELP);
        return Ok(());
    }

    // Verbose flag
    let verbose = args.contains(["-v", "--verbose"]);
    let log_level = if verbose { Level::DEBUG } else { Level::INFO };

    // Initialize tracing
    let subscriber = FmtSubscriber::builder().with_max_level(log_level).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Positional arguments
    let id: u64 = match args.free_from_str() {
        Ok(id) => id,
        Err(_) => {
            error!("Missing or invalid <set_id> argument.");
            print!("{}", HELP);
            std::process::exit(87);
        }
    };

    // If err, means not set, use OK to cast to None
    let base_folder: Option<PathBuf> = args.free_from_str().ok();

    // Run the async workflow
    smol::block_on(Compat::new(async {
        if let Err(e) = download_set(id, base_folder).await {
            error!("Failed to download sticker set: {}", e);
            std::process::exit(1);
        }
        Ok(())
    }))
}
