mod utils;
mod ncolor;

use std::fs::{File, create_dir_all, rename};
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use reqwest::blocking::get;

use crate::utils::colors::*;

const INPUT_VIDEO: &str = "input/badapple.mov";
const OUTPUT_VIDEO: &str = "output/badapple.mp4";

const BAD_APPLE_VIDEO_URLS: &[&str] = &[
    "https://badapple.mov/badapple.mov",
    "https://web.archive.org/web/20260826212812/https://badapple.mov/badapple.mov",
];

const NCOLOR_CONFIG: ncolor::NColorConfig = ncolor::NColorConfig {
    #[rustfmt::skip]
    palette: &[
        BLACK, 
        WHITE,
        TEAL,
        // ORANGE,
        // PURPLE,
        // PINK,
        // RED,
        // ORANGE2,
        // YELLOW,
        // GREEN,
        // CYAN,
        // BLUE,
        // INDIGO,
        // VIOLET,
    ],
    pixel_heal: Some(ncolor::NColorPixelHeal {
        frames: 50,
        fade: true,
    })
};
const _: () = assert!(NCOLOR_CONFIG.palette.len() >= 2);

fn ensure_dir_for_file(path: &Path) -> Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    create_dir_all(parent)?;
    return Ok(());
}

fn ensure_bad_apple_video_file(path: &Path) -> Result<()> {
    if path.try_exists()? {
        return Ok(());
    }

    let temp_path = path.with_added_extension("tmp");
    let mut problems: Vec<String> = vec![];
    for url in BAD_APPLE_VIDEO_URLS {
        let mut response = match get(*url).and_then(|r| r.error_for_status()) {
            Ok(r) => r,
            Err(e) => {
                problems.push(e.to_string());
                continue;
            }
        };

        let mut file = File::create(&temp_path)?;
        response.copy_to(&mut file)?;
        rename(&temp_path, path)?;
        return Ok(());
    }

    if !problems.is_empty() {
        bail!("failed to download file:\n{}", problems.join("\n"));
    }
    bail!("failed to download file: no download URLs configured");
}

fn main() -> Result<()> {
    let input_path = Path::new(INPUT_VIDEO);
    let output_path = Path::new(OUTPUT_VIDEO);
    let start_time = Instant::now();

    ensure_dir_for_file(input_path).context("failed to ensure input dir")?;
    ensure_dir_for_file(output_path).context("failed to ensure output dir")?;
    ensure_bad_apple_video_file(input_path).context("failed to ensure Bad Apple video file")?;
    ncolor::process_video(&NCOLOR_CONFIG, input_path, output_path).context("failed to process video")?;

    let delta = start_time.elapsed();
    println!("Video processed successfully in {delta:?}: {output_path:?}");

    Ok(())
}
