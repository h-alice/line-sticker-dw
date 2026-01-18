use std::path::PathBuf;

use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

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
    let response = reqwest::get(sticker_page_url(id)).await?;
    let text = response.text().await?;
    let parsed_stickers = parse_stickers(&text)?;
    if parsed_stickers.is_empty() {
        return Err(StickerError::NoStickersFound);
    }
    Ok(parsed_stickers)
}

/// Downloads the sticker image to the local filesystem.
///
/// The sticker image is selected based on availability in the following priority order:
/// 1. Sound sticker
/// 2. Animation sticker
/// 3. Static sticker
/// 4. Fallback static sticker PNG
///
/// ## Returns
/// Returns the path to the downloaded sticker image.
///
/// The sticker image is saved as `sticker_<sticker id>.png`.
pub async fn download_sticker_image(sticker: &StickerPreview) -> Result<PathBuf, StickerError> {
    let url = if sticker.has_sound() {
        &sticker.sound_url
    } else if sticker.has_animation() {
        &sticker.animation_url
    } else if sticker.has_static() {
        &sticker.static_url
    } else if sticker.has_fallback_static() {
        &sticker.fallback_static_url
    } else {
        return Err(StickerError::InvalidSticker);
    };

    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;
    let filename = format!("sticker_{}.png", sticker.id);
    std::fs::write(&filename, bytes)?;
    Ok(PathBuf::from(filename))
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
