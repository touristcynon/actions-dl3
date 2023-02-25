use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::time::Duration;

use tencent3::{
    hyper::client::HttpConnector, hyper_rustls::HttpsConnector, Credential, TencentClient,
};
use tokio::fs;

use super::subtitle::SubtitleStream;

pub const SPLIT_LABEL: &str = "🍎";

type Tmt = TencentClient<HttpsConnector<HttpConnector>>;

async fn translate_srt_file(
    srt_file: impl AsRef<Path>,
    lang: &str,
    translator: &Tmt,
) -> Result<(), io::Error> {
    let mut subtitle_stream = SubtitleStream::load_from_file(srt_file.as_ref())?;
    let target = if lang.contains("zh") { "en" } else { "zh" };
    let mut subtitle_map: HashMap<String, Vec<u16>> = HashMap::new();
    let mut to_translate = String::new();
    for sub in subtitle_stream.iter() {
        let text = sub.text.as_str();
        // 如果前一行跟当前行字幕内容相同，就不加入待翻译字符串，为了确保译文跟原文对应，需要设置`🍎`分隔符
        assert_ne!(sub.index, 0);
        let pat = &['\n', '\r', '\t'];
        let text = if text.len() < 36 && text.contains(pat) {
            text.trim().replace(pat, " ")
        } else {
            text.to_string()
        };
        match subtitle_map.get(text.as_str()) {
            Some(idxs) if idxs.contains(&(sub.index - 1)) => (),
            _ => {
                to_translate.push_str(text.as_str());
                to_translate.push_str(SPLIT_LABEL);
            }
        }
        subtitle_map.entry(text).or_default().push(sub.index);
    }
    // TMT, Baidu quota is 2000
    let translated = if to_translate.len() < 2000 {
        text_translate(translator, &to_translate, target).await?
    } else {
        let subtitles = split_text_by_word(2000, &to_translate);
        let mut translated_text = String::new();
        for sub in subtitles {
            let translated = text_translate(translator, sub, target).await?;
            translated_text.push_str(&translated);
            tokio::time::sleep(Duration::from_secs(6)).await;
        }
        translated_text
    };
    for (part_translated, part_translate) in translated
        .split(SPLIT_LABEL)
        .zip(to_translate.split(SPLIT_LABEL))
    {
        if part_translated.is_empty() {
            continue;
        }
        let pat = &['\n', '\r', '\t'];
        let max_length = if target.contains("en") { 36 } else { 56 };
        let part_translated =
            if part_translated.trim().len() < max_length && part_translated.trim().contains(pat) {
                part_translated.trim().replace(pat, " ")
            } else {
                part_translated.trim().to_string()
            };

        let idxs = subtitle_map
            .remove(part_translate)
            .unwrap_or_else(|| panic!("none {}", part_translate));
        for idx in idxs {
            let sub = subtitle_stream.get_mut(idx as usize).unwrap();
            sub.text = format!("{}\n{}", part_translated, part_translate);
        }
    }

    fs::write(srt_file.as_ref(), format!("{subtitle_stream}")).await?;
    Ok(())
}

async fn text_translate(translator: &Tmt, text: &str, target: &str) -> Result<String, io::Error> {
    let Ok(project_id) = std::env::var("TENCENT_PROJECT_ID") else {
        return Err(io::Error::new(io::ErrorKind::Other, "project id not exist"));
    };
    let Ok(region) = std::env::var("TENCENT_REGION") else {
        return Err(io::Error::new(io::ErrorKind::Other, "region not exist"));
    };
    let call = translator
        .translate()
        .text_translate()
        .project_id(project_id.parse().unwrap())
        .region(region.as_str())
        .source("auto")
        .untranslated_text(SPLIT_LABEL)
        .target(target)
        .source_text(text)
        .build()
        .unwrap();
    let translated_text = call
    .doit(|result| {
        let Ok(string) = String::from_utf8(result) else {
            return String::new();
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&string) else {
            return String::new();
        };
        let Some(text) = value.get("Response").and_then(|res| res.get("TargetText")).and_then(|e| e.as_str()) else {
            return String::new();
        };
        text.to_string()
    })
    .await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    Ok(translated_text)
}

/// 字符串按照字节数分割，分割点在不大于字节数前面的空格分割点
fn split_text_by_word(index: usize, mut remaining: &str) -> Vec<&str> {
    let mut result = vec![];
    while remaining.len() > index {
        let start = {
            let lower_bound = index.saturating_sub(3);
            let new_index = remaining.as_bytes()[lower_bound..=index]
                .iter()
                .rposition(|&b| (b as i8) >= -0x40);

            // SAFETY: we know that the character boundary will be within four bytes
            unsafe { lower_bound + new_index.unwrap_unchecked() }
        };
        let (middle, _) = &remaining[..start]
            .char_indices()
            .rfind(|(_, c)| c.is_whitespace())
            .unwrap_or_default();
        let (s, r) = remaining.split_at(*middle);
        remaining = r;
        if s.is_empty() {
            continue;
        }
        result.push(s);
    }

    if !remaining.is_empty() {
        result.push(remaining);
    }

    result
}

pub async fn batch_translate(dir_path: &str) {
    let Ok(mut entries) = fs::read_dir(dir_path).await else {
        return;
    };
    let mut srt_files = vec![];
    while let Ok(Some(entry)) = entries.next_entry().await {
        let file_path = entry.path();
        if let Some(extension) = file_path.extension() {
            if extension == "srt" {
                srt_files.push(file_path);
            }
        }
    }

    let Ok(id) = std::env::var("TENCENT_ID") else {
        return;
    };
    let Ok(key) = std::env::var("TENCENT_KEY") else {
        return;
    };
    let credential = Credential { key, id };

    let translator = TencentClient::native(credential);
    for file_path in srt_files {
        let lang = file_path
            .file_stem()
            .map(|stem| Path::new(stem))
            .and_then(|parent| parent.extension())
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        if let Err(e) = translate_srt_file(file_path.as_path(), lang, &translator).await {
            eprintln!("{e}");
        }
    }
}
