use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn insert_subtitle(dir_path: &str) {
    // 用一个 HashMap 存储同类 A 文件的文件名和路径
    let mut srt_files = Vec::new();
    let mut all_files = Vec::new();

    for entry in fs::read_dir(dir_path).unwrap() {
        let path = entry.unwrap().path();
        let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
            continue;
        };
        if ext == "srt" {
            let mut base_name = path.with_extension("");
            base_name.set_extension("");
            srt_files.push((
                base_name
                    .file_stem()
                    .and_then(|c| c.to_str())
                    .unwrap()
                    .to_string(),
                path,
            ));
            continue;
        }
        all_files.push(path);
    }
    // 对于每个同类 A 文件，查找同类 B 文件并打印路径

    for (stem, srt_path) in srt_files {
        let find_file_strem = |&c: &&PathBuf| {
            if c.file_stem().is_none() {
                return false;
            }
            if c.file_stem().and_then(|c| c.to_str()).unwrap() == stem {
                return true;
            }
            false
        };
        let Some(video_path) = all_files.iter().find(find_file_strem) else {
            continue;
        };
        let filename = format!(
            "{}_finish.{}",
            video_path.file_stem().and_then(|c| c.to_str()).unwrap(),
            video_path.extension().and_then(|c| c.to_str()).unwrap()
        );
        let output_path = video_path.with_file_name(filename);
        println!("do {}", output_path.display());
        let Ok(mut child) = Command::new("ffmpeg")
        .args([
            "-i",
            video_path.to_str().unwrap(),
            "-vf",
            format!("subtitles={}", srt_path.to_str().unwrap()).as_str(),
            output_path.to_str().unwrap(),
            "-y",
        ]).spawn() else {
            continue;
        };
        let Ok(exit_code) =  child.wait() else {
            continue;
        };
        if exit_code.success() {
            let _ = fs::remove_file(video_path);
        }
    }
}
