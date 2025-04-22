use hhkodo::{parse_frags, Frag};

use crate::MessageFragment;

pub fn parse_bbcode(input: &str) -> Vec<MessageFragment> {
    let frags = parse_frags(input);
    frags_to_message(&frags)
}

fn frags_to_message(frags: &[Frag]) -> Vec<MessageFragment> {
    let mut out = Vec::new();
    for frag in frags {
        match frag {
            Frag::Raw(text) => {
                if !text.is_empty() {
                    out.push(MessageFragment::Text(text.clone()));
                }
            }
            Frag::Tag {
                name,
                val,
                subfrags,
                ..
            } => {
                let tag = name.to_lowercase();
                match tag.as_str() {
                    "img" | "image" => {
                        if let Some(mut url) = extract_raw(subfrags) {
                            if url.starts_with("//") {
                                url = format!("https:{}", &url);
                            }
                            let mime = mime_from_extension(&url);
                            out.push(MessageFragment::Image { url, mime });
                        } else {
                            out.extend(frags_to_message(subfrags));
                        }
                    }
                    "video" => {
                        if let Some(mut url) = extract_raw(subfrags) {
                            if url.starts_with("//") {
                                url = format!("https:{}", &url);
                            }
                            let mime = mime_from_extension(&url);
                            out.push(MessageFragment::Video { url, mime });
                        } else {
                            out.extend(frags_to_message(subfrags));
                        }
                    }
                    "audio" => {
                        if let Some(mut url) = extract_raw(subfrags) {
                            if url.starts_with("//") {
                                url = format!("https:{}", &url);
                            }
                            let mime = mime_from_extension(&url);
                            out.push(MessageFragment::Audio { url, mime });
                        } else {
                            out.extend(frags_to_message(subfrags));
                        }
                    }
                    "url" => {
                        let link = val.clone().or_else(|| extract_raw(subfrags));
                        if let Some(mut href) = link {
                            if href.starts_with("//") {
                                href = format!("https:{}", &href);
                            }
                            out.push(MessageFragment::Url(href));
                        } else {
                            out.extend(frags_to_message(subfrags));
                        }
                    }
                    _ => {
                        out.extend(frags_to_message(subfrags));
                    }
                }
            }
        }
    }
    out
}

fn extract_raw(subfrags: &[Frag]) -> Option<String> {
    if subfrags.len() == 1 {
        if let Frag::Raw(text) = &subfrags[0] {
            return Some(text.clone());
        }
    }
    None
}

fn mime_from_extension(url: &str) -> String {
    if let Some(ext) = url.split('.').last().map(|s| s.to_lowercase()) {
        match ext.as_str() {
            // images
            "png" => "image/png".into(),
            "jpg" | "jpeg" => "image/jpeg".into(),
            "gif" => "image/gif".into(),
            "webp" => "image/webp".into(),
            // video
            "mp4" => "video/mp4".into(),
            "webm" => "video/webm".into(),
            "ogv" => "video/ogg".into(),
            // audio
            "mp3" => "audio/mpeg".into(),
            "wav" => "audio/wav".into(),
            "flac" => "audio/flac".into(),
            "oga" | "ogg" => "audio/ogg".into(),
            _ => default_mime(url),
        }
    } else {
        default_mime(url)
    }
}

fn default_mime(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        "application/octet-stream".into()
    } else {
        "text/plain".into()
    }
}
