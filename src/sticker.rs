use std::path::PathBuf;

use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(thiserror::Error, Debug)]
pub enum StickerError {
    // HTTP errors
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    // THe returned sticker page has no stickers
    #[error("No stickers found")]
    NoStickersFound,

    // The sticker entry has no valid image URL
    #[error("Invalid sticker")]
    InvalidSticker,

    // Parse error
    #[error("Parse error: {0}")]
    ParseError(String),

    // JSON error, from `serde_json`
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    // File system error
    #[error("FileSystem error: {0}")]
    FileSystem(#[from] std::io::Error),
}

/// A sticker entry
///
/// We follow the JSON structure of LINE sticker page.
///
/// It may change in the future, we cannot ensure future
/// compatibility. (you can file an issue if it breaks)
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StickerPreview {
    #[serde(rename = "type")]
    pub sticker_type: String,
    pub id: String,
    pub static_url: String,
    pub fallback_static_url: String,
    pub animation_url: String,
    pub popup_url: String,
    pub sound_url: String,
}

impl StickerPreview {
    /// Determine if the sticker has a static image
    ///
    /// "Static image" is the default sticker type of LINE stickers,
    /// just a plain PNG file.
    ///
    /// In general, "every" sticker would have a static image, even if
    /// it is an animated sticker.
    pub fn has_static(&self) -> bool {
        !self.static_url.is_empty()
    }

    /// Determine if the sticker has an animation
    ///
    /// So far (2026), the animated sticker uses APNG format. It may looks
    /// like a PNG, but much bigger, and it will be truncated to normal PNG
    /// if upload to some platform like Discord, so take care!
    pub fn has_animation(&self) -> bool {
        !self.animation_url.is_empty()
    }

    /// Determine if the sticker has a fallback static image
    pub fn has_fallback_static(&self) -> bool {
        !self.fallback_static_url.is_empty()
    }

    /// Determine if the sticker has a sound
    ///
    /// NOTE: Not tried yet! Any recommendation for sound stickers?
    pub fn has_sound(&self) -> bool {
        !self.sound_url.is_empty()
    }
}

/// Returns the URL of the sticker page of given id
///
/// # Arguments
///
/// * `id` - The id of the sticker "set" (not sticker itself!)
///
/// # Returns
///
/// The URL of the sticker page
pub fn sticker_page_url(id: u64) -> String {
    format!("https://store.line.me/stickershop/product/{}", id)
}

/// Parses the stickers from the sticker page
///
/// # Arguments
///
/// * `html` - The HTML content of the sticker page
///
/// # Returns
///
/// The sticker entries (in `StickerPreview`) of the sticker page
pub fn parse_stickers(html: &str) -> Result<Vec<StickerPreview>, StickerError> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(r#".FnStickerPreviewItem"#)
        .map_err(|e| StickerError::ParseError(e.to_string()))?;

    let stickers_elements = document.select(&selector);
    let mut stickers = Vec::new();

    for sticker in stickers_elements {
        if let Some(preview) = sticker.attr("data-preview") {
            let info: StickerPreview = serde_json::from_str(preview)?;
            stickers.push(info);
        }
    }

    if stickers.is_empty() {
        return Err(StickerError::NoStickersFound);
    }

    Ok(stickers)
}

/// Fetches the stickers from the sticker page of given id
///
/// # Arguments
///
/// * `id` - The id of the sticker "set" (not sticker itself!)
///
/// # Returns
///
/// The stickers of the sticker page
pub async fn fetch_stickers(id: u64) -> Result<Vec<StickerPreview>, StickerError> {
    debug!("Downloading sticker store page for set {}", id);
    let response = reqwest::get(sticker_page_url(id)).await?;
    let text = response.text().await?;
    debug!("Store page downloaded for set {}", id);
    let parsed_stickers = parse_stickers(&text)?;
    if parsed_stickers.is_empty() {
        return Err(StickerError::NoStickersFound);
    }
    Ok(parsed_stickers)
}

/// Helper to download a file from a URL to a specific destination.
async fn download_file(url: &str, dest: PathBuf) -> Result<(), StickerError> {
    if let Some(parent) = dest.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;
    std::fs::write(&dest, bytes)?;
    Ok(())
}

/// Downloads the sticker image (and sound if it has one) to the local filesystem.
///
/// The sticker image is selected based on availability in the following priority order:
/// 1. Animation sticker
/// 2. Static sticker
/// 3. Fallback static sticker PNG
///
/// If the sticker has a sound, it will also be downloaded as a `.m4a` file.
///
/// ## Arguments
///
/// * `sticker` - The sticker to download
/// * `base` - The base directory to save the sticker files
///
/// The `base` directory is optional, if not provided, the sticker
/// image will be saved to the current directory.
///
/// ## Returns
/// Returns the path to the downloaded sticker image.
///
/// The sticker image is saved as `sticker_<sticker id>.png`. \
/// The sound is saved as `sticker_<sticker id>.m4a` (if it has one).
pub async fn download_sticker_image(
    sticker: &StickerPreview,
    base: Option<PathBuf>,
) -> Result<PathBuf, StickerError> {
    debug!("downloading sticker {}", sticker.id);

    let image_url = if sticker.has_animation() {
        &sticker.animation_url
    } else if sticker.has_static() {
        &sticker.static_url
    } else if sticker.has_fallback_static() {
        &sticker.fallback_static_url
    } else {
        return Err(StickerError::InvalidSticker);
    };

    let base_dir = base.unwrap_or_default();
    let image_path = base_dir.clone().join(format!("sticker_{}.png", sticker.id));

    // Download the image part
    download_file(image_url, image_path.clone()).await?;

    // Download the sound part if it has sound
    if sticker.has_sound() {
        let sound_path = base_dir.clone().join(format!("sticker_{}.m4a", sticker.id));
        download_file(&sticker.sound_url, sound_path).await?;
    }

    debug!("Downloaded sticker {}.", sticker.id);
    Ok(image_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialization() {
        let json = r#"{ 
        "type" : "static", 
        "id" : "655976027", 
        "staticUrl" : "https://stickershop.line-scdn.net/stickershop/v1/sticker/655976027/android/sticker.png?v=1", 
        "fallbackStaticUrl" : "https://stickershop.line-scdn.net/stickershop/v1/sticker/655976027/android/sticker.png?v=1", 
        "animationUrl" : "", 
        "popupUrl" : "", 
        "soundUrl" : "" }"#;

        let preview: StickerPreview = serde_json::from_str(json).unwrap();

        assert_eq!(preview.sticker_type, "static");
        assert_eq!(preview.id, "655976027");
        assert_eq!(
            preview.static_url,
            "https://stickershop.line-scdn.net/stickershop/v1/sticker/655976027/android/sticker.png?v=1"
        );
        assert!(preview.has_static());
        assert!(!preview.has_animation());
        assert!(preview.has_fallback_static());
    }
}
