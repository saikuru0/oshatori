use crate::{Asset, MessageFragment};
use regex::Regex;

pub fn parse_assets(text: &str, assets: &[Asset]) -> Vec<MessageFragment> {
    let mut frags = Vec::new();
    let mut current_text = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let remaining: String = chars[i..].iter().collect();
        let mut found_match = false;

        for asset in assets {
            let pattern = get_pattern(asset);
            if let Ok(regex) = Regex::new(&pattern) {
                if let Some(mat) = regex.find(&remaining) {
                    if mat.start() == 0 {
                        if !current_text.is_empty() {
                            frags.push(MessageFragment::Text(current_text.clone()));
                            current_text.clear();
                        }

                        if let Some(id) = get_id(asset) {
                            frags.push(MessageFragment::AssetId(id));
                        }

                        i += mat.end();
                        found_match = true;
                        break;
                    }
                }
            }
        }

        if !found_match {
            current_text.push(chars[i]);
            i += 1;
        }
    }

    if !current_text.is_empty() {
        frags.push(MessageFragment::Text(current_text));
    }

    merge_text_frags(frags)
}

fn get_pattern(asset: &Asset) -> String {
    match asset {
        Asset::Emote { pattern, .. } => pattern.clone(),
        Asset::Sticker { pattern, .. } => pattern.clone(),
        Asset::Audio { pattern, .. } => pattern.clone(),
        Asset::Command { pattern, .. } => pattern.clone(),
    }
}

fn get_id(asset: &Asset) -> Option<String> {
    match asset {
        Asset::Emote { id, .. } => id.clone(),
        Asset::Sticker { id, .. } => id.clone(),
        Asset::Audio { id, .. } => id.clone(),
        Asset::Command { id, .. } => id.clone(),
    }
}

fn merge_text_frags(fragments: Vec<MessageFragment>) -> Vec<MessageFragment> {
    let mut result = Vec::new();
    let mut current_text = String::new();

    for fragment in fragments {
        match fragment {
            MessageFragment::Text(text) => {
                current_text.push_str(&text);
            }
            other => {
                if !current_text.is_empty() {
                    result.push(MessageFragment::Text(current_text.clone()));
                    current_text.clear();
                }
                result.push(other);
            }
        }
    }

    if !current_text.is_empty() {
        result.push(MessageFragment::Text(current_text));
    }

    result
}
