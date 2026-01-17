
/// Returns the URL of the sticker page
/// 
pub fn sticker_page_url(id: u64) -> String {
    format!("https://store.line.me/stickershop/product/{}", id)
}

