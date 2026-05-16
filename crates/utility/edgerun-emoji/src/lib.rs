//! Emoji assets and raster helpers for Edgerun text interfaces.

mod generated;

pub use generated::EMOJI_ASSETS;

pub const SOURCE_SIZE: usize = 64;

pub struct EmojiAsset {
    pub sequence: &'static str,
    pub source: &'static [u8],
}

pub fn longest_match_prefix(input: &str) -> Option<&'static EmojiAsset> {
    EMOJI_ASSETS
        .iter()
        .find(|asset| input.starts_with(asset.sequence))
}

pub fn asset_for_char(ch: char) -> Option<&'static EmojiAsset> {
    let mut buf = [0u8; 4];
    let text = ch.encode_utf8(&mut buf);
    EMOJI_ASSETS.iter().find(|asset| {
        asset.sequence == text || asset.sequence.strip_suffix('\u{fe0f}') == Some(text)
    })
}

pub fn is_supported_emoji(ch: char) -> bool {
    asset_for_char(ch).is_some()
}

pub fn rasterize_char(ch: char, width: u32, height: u32) -> Option<Vec<u8>> {
    let asset = asset_for_char(ch)?;
    Some(scale_rgba(
        asset.source,
        SOURCE_SIZE,
        SOURCE_SIZE,
        width,
        height,
    ))
}

pub fn rasterize_sequence(sequence: &str, width: u32, height: u32) -> Option<Vec<u8>> {
    let asset = EMOJI_ASSETS.iter().find(|asset| {
        asset.sequence == sequence || asset.sequence.strip_suffix('\u{fe0f}') == Some(sequence)
    })?;
    Some(scale_rgba(
        asset.source,
        SOURCE_SIZE,
        SOURCE_SIZE,
        width,
        height,
    ))
}

fn scale_rgba(
    source: &[u8],
    source_width: usize,
    source_height: usize,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let width = width.max(1) as usize;
    let height = height.max(1) as usize;
    let mut out = vec![0u8; width.saturating_mul(height).saturating_mul(4)];
    for y in 0..height {
        let sy = y.saturating_mul(source_height) / height;
        for x in 0..width {
            let sx = x.saturating_mul(source_width) / width;
            let src = (sy * source_width + sx) * 4;
            let dst = (y * width + x) * 4;
            out[dst..dst + 4].copy_from_slice(&source[src..src + 4]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{is_supported_emoji, longest_match_prefix, rasterize_char, rasterize_sequence};

    #[test]
    fn rasterizes_supported_emoji() {
        assert!(is_supported_emoji('😊'));
        let rgba = rasterize_char('😊', 20, 18).unwrap();
        assert_eq!(rgba.len(), 20 * 18 * 4);
        assert!(rgba.chunks_exact(4).any(|px| px[3] > 0));
    }

    #[test]
    fn matches_sequences() {
        let text = "👩🏽‍🚒 ok";
        let matched = longest_match_prefix(text).unwrap();
        assert_eq!(matched.sequence, "👩🏽‍🚒");
        let rgba = rasterize_sequence(matched.sequence, 20, 20).unwrap();
        assert_eq!(rgba.len(), 20 * 20 * 4);
    }
}
