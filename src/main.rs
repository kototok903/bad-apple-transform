use std::fs::{File, rename};
use std::path::Path;
use std::error::Error;

use reqwest::blocking::get;

const BAD_APPLE_VIDEO: &str = "input/badapple.mov";

const BAD_APPLE_VIDEO_URLS: &[&str] = &[
    "https://badapple.mov/badapple.mov",
    "https://web.archive.org/web/20260826212812/https://badapple.mov/badapple.mov",
];

fn ensure_bad_apple_file(path_str: &str) -> Result<(), Box<dyn Error>> {
    let path = Path::new(path_str);

    if path.try_exists()? {
        return Ok(());
    }

    let temp_path_str = format!("{}_temp", path_str);
    let temp_path = Path::new(&temp_path_str);
    for url in BAD_APPLE_VIDEO_URLS {
        let Ok(response) = get(*url) else { continue };
        let Ok(mut response) = response.error_for_status() else { continue };

        let mut file = File::create(temp_path)?;
        response.copy_to(&mut file)?;
        rename(temp_path, path)?;
        return Ok(());
    }

    Err("failed to download file".into())
}

fn main() {
    if let Err(e) = ensure_bad_apple_file(BAD_APPLE_VIDEO) {
        eprintln!("Failed to ensure Bad Apple file: {e}")
    }

    

    println!("Hello, world!");
}
