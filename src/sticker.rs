use crate::line_url_maker::sticker_page_url;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug)]
pub enum StickerError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("No stickers found")]
    NoStickersFound,
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

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

pub async fn fetch_stickers(id: u64) -> Result<Vec<StickerPreview>, StickerError> {
    let response = reqwest::get(sticker_page_url(id)).await?;
    let text = response.text().await?;
    parse_stickers(&text)
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
    }
}
