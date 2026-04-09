use const_format::concatcp;
use regex::{Regex, RegexBuilder};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Deserialize)]
struct VscordData {
    #[serde(rename = "KNOWN_EXTENSIONS")]
    known_extensions: HashMap<String, AssetNode>,
}

#[derive(Deserialize)]
struct AssetNode {
    image: String,
}

struct AssetMap {
    exact_extensions: HashMap<String, String>,
    regex_patterns: Vec<(Regex, String)>,
}

const ASSET_BASE_URL: &str =
    "https://raw.githubusercontent.com/norinorin/helix-discord-rpc/refs/heads/main/assets/icons/";
pub const HELIX_ICON_URL: &str = concatcp!(ASSET_BASE_URL, "helix.png");
pub const IDLE_ICON_URL: &str = concatcp!(ASSET_BASE_URL, "idle.png");

static ASSET_MAP: LazyLock<AssetMap> = LazyLock::new(|| {
    let raw_json = include_str!("../assets/vscord-languages.json");
    let data: VscordData = serde_json::from_str(raw_json).expect("Invalid JSON");

    let mut regex_patterns = Vec::new();
    let mut exact_extensions = HashMap::new();

    for (pattern, node) in data.known_extensions {
        if pattern.starts_with('/') {
            let parts: Vec<&str> = pattern.split('/').collect();
            if parts.len() >= 3 {
                let raw_regex = parts[1];
                let is_case_insensitive = parts[2].contains('i');
                if let Ok(re) = RegexBuilder::new(raw_regex)
                    .case_insensitive(is_case_insensitive)
                    .build()
                {
                    regex_patterns.push((re, node.image));
                }
            }
        } else {
            exact_extensions.insert(pattern.to_lowercase(), node.image);
        }
    }

    AssetMap {
        exact_extensions,
        regex_patterns,
    }
});

pub fn get_asset_url(filename: &str) -> String {
    let map = &*ASSET_MAP;

    let mut matched_asset = None;
    for (re, asset) in &map.regex_patterns {
        if re.is_match(filename) {
            matched_asset = Some(asset.as_str());
            break;
        }
    }

    let final_asset = matched_asset.unwrap_or_else(|| {
        let ext = std::path::Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        map.exact_extensions
            .get(&ext)
            .map(|s| s.as_str())
            .unwrap_or("text")
    });

    format!("{}{}.png", ASSET_BASE_URL, final_asset)
}
