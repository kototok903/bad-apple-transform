use std::fs::{File, create_dir_all, rename};
use std::io::Write;
use std::path::Path;
use std::thread;
use std::time::Instant;

use anyhow::{Context, Result, anyhow, bail};
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::{FfmpegEvent, LogLevel, OutputVideoFrame};
use reqwest::blocking::get;

const INPUT_VIDEO: &str = "input/badapple.mov";
const OUTPUT_VIDEO: &str = "output/badapple.mp4";

const BAD_APPLE_VIDEO_URLS: &[&str] = &[
    "https://badapple.mov/badapple.mov",
    "https://web.archive.org/web/20260826212812/https://badapple.mov/badapple.mov",
];

const BLACK: [u8; 3] = [0x00, 0x00, 0x00];
const WHITE: [u8; 3] = [0xFF, 0xFF, 0xFF];
const TEAL: [u8; 3] = [0x06, 0x94, 0x94];
const ORANGE: [u8; 3] = [0xFC, 0x6A, 0x03];
const PURPLE: [u8; 3] = [0x66, 0x34, 0xB8];
const PINK: [u8; 3] = [0xFF, 0xC0, 0xCB];
// const RED:    [u8; 3] = [0xFF, 0x00, 0x00];
// const ORANGE: [u8; 3] = [0xFF, 0x7F, 0x00];
// const YELLOW: [u8; 3] = [0xFF, 0xFF, 0x00];
// const GREEN:  [u8; 3] = [0x00, 0xFF, 0x00];
// const CYAN:   [u8; 3] = [0x00, 0xFF, 0xFF];
// const BLUE:   [u8; 3] = [0x00, 0x00, 0xFF];
// const INDIGO: [u8; 3] = [0x4B, 0x00, 0x82];
// const VIOLET: [u8; 3] = [0x94, 0x00, 0xD3];

// Edit this to change what colors are used and in what order
const OUTPUT_COLORS: &[[u8; 3]] = &[
    BLACK, WHITE, TEAL, ORANGE, PURPLE,
    PINK,
    // RED,
    // ORANGE,
    // YELLOW,
    // GREEN,
    // CYAN,
    // BLUE,
    // INDIGO,
    // VIOLET
];

// After this number of frames the pixel goes back to its original (B or W) color,
// set to None to disable.
const HEAL_AFTER_FRAMES: Option<u32> = Some(50);

fn ensure_dir_for_file(path_str: &str) -> Result<()> {
    let path = Path::new(path_str);
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    create_dir_all(parent)?;
    return Ok(());
}

fn ensure_bad_apple_video_file(path_str: &str) -> Result<()> {
    let path = Path::new(path_str);

    if path.try_exists()? {
        return Ok(());
    }

    let temp_path_str = format!("{}_temp", path_str);
    let temp_path = Path::new(&temp_path_str);
    for url in BAD_APPLE_VIDEO_URLS {
        let Ok(response) = get(*url) else { continue };
        let Ok(mut response) = response.error_for_status() else {
            continue;
        };

        let mut file = File::create(temp_path)?;
        response.copy_to(&mut file)?;
        rename(temp_path, path)?;
        return Ok(());
    }

    bail!("failed to download file");
}

#[derive(Clone)]
struct PixelState {
    is_black: bool,
    color_idx: usize,
    last_updated: u32,
}

fn is_black(val: u8) -> bool {
    return val < 128;
}

fn process_frames<T: Iterator<Item = OutputVideoFrame>, S: Write>(
    frames_iter: T,
    pixel_info: &mut Vec<PixelState>,
    output: &mut S,
    width: u32,
    height: u32,
) -> Result<()> {
    let mut output_buffer = Vec::with_capacity((width * height * 3) as usize); // u8 to u24 pixels
    let (heal_frames, can_heal) = match HEAL_AFTER_FRAMES {
        Some(h) => (h, true),
        None => (0, false),
    };

    for frame in frames_iter {
        output_buffer.clear();
        for (info, val) in pixel_info.iter_mut().zip(frame.data) {
            if info.is_black != is_black(val) {
                info.is_black = !info.is_black;
                info.color_idx = (info.color_idx + 1) % OUTPUT_COLORS.len();
                info.last_updated = frame.frame_num;
            }
            if can_heal && frame.frame_num > info.last_updated + heal_frames {
                info.color_idx = if info.is_black { 0 } else { 1 }; // assuming BLACK is the first element in OUTPUT_COLORS
            }
            output_buffer.extend(OUTPUT_COLORS[info.color_idx]);
        }
        output.write_all(&output_buffer)?;
    }

    Ok(())
}

fn process_video(in_path_str: &str, out_path_str: &str) -> Result<()> {
    // create video decoder (input)
    let mut ffmpeg_decoder = FfmpegCommand::new()
        .input(in_path_str)
        .format("rawvideo")
        .pix_fmt("gray")
        .pipe_stdout()
        .spawn()
        .context("failed to spawn ffmpeg decoder")?;

    // extract metadata
    let mut events_iter = ffmpeg_decoder.iter()?;
    let metadata = events_iter.collect_metadata()?;
    let frames_iter = events_iter.filter_frames();
    let stream_data = metadata
        .output_streams
        .iter()
        .find_map(|s| s.video_data())
        .context("no video data in input video metadata")?;

    let (width, height) = (stream_data.width, stream_data.height);
    let fps = stream_data.fps;
    let duration_sec = metadata.duration().unwrap_or(0f64) as i32;
    let mut pixel_info = vec![
        PixelState {
            is_black: true,
            color_idx: 0,
            last_updated: 0
        };
        (width * height) as usize
    ];

    // create video encoder (output)
    let mut ffmpeg_encoder = FfmpegCommand::new()
        // video input (frames from stdin)
        .format("rawvideo")
        .pix_fmt("rgb24")
        .size(width, height)
        .rate(fps)
        .input("-")
        // audio input
        .input(in_path_str)
        // output
        .codec_audio("copy")
        .codec_video("libx264")
        .pix_fmt("yuv420p")
        .map("0:v")
        .map("1:a")
        .overwrite()
        .output(out_path_str)
        .spawn()
        .context("failed to spawn ffmpeg encoder")?;

    let mut encoder_stdin = ffmpeg_encoder
        .take_stdin()
        .context("failed to take output stdin")?;
    let encoder_logs_iter = ffmpeg_encoder.iter()?;

    let output_worker = thread::spawn(move || {
        let mut problems = Vec::new();
        for e in encoder_logs_iter {
            match e {
                FfmpegEvent::Log(LogLevel::Error | LogLevel::Fatal, e) => {
                    problems.push(e);
                }
                FfmpegEvent::Progress(p) => println!(
                    "Progress: F{}\t{} / {:02}:{:02}:{:02}",
                    p.frame,
                    p.time,
                    duration_sec / 3600,
                    duration_sec / 60 % 60,
                    duration_sec % 60
                ),
                _ => {}
            }
        }
        problems
    });

    let processing_res = process_frames(
        frames_iter,
        &mut pixel_info,
        &mut encoder_stdin,
        width,
        height,
    );

    // release all resources
    drop(encoder_stdin);
    let output_worker_res = output_worker.join();
    let output_status = ffmpeg_encoder.wait();
    let input_status = ffmpeg_decoder.wait();

    // handle errors
    let problems = output_worker_res.map_err(|_| anyhow!("output logs thread error"))?;
    let output_status = output_status?;
    if !output_status.success() {
        bail!(
            "output thread exited with status {}, logged errors:\n{}",
            output_status,
            problems.join("\n")
        );
    }
    let input_status = input_status?;
    if !input_status.success() {
        bail!("input thread exited with status {input_status}");
    }
    processing_res?;

    Ok(())
}

fn main() -> Result<()> {
    let start_time = Instant::now();

    ensure_dir_for_file(INPUT_VIDEO).context("failed to ensure input dir")?;
    ensure_dir_for_file(OUTPUT_VIDEO).context("failed to ensure output dir")?;
    ensure_bad_apple_video_file(INPUT_VIDEO).context("failed to ensure Bad Apple video file")?;
    process_video(INPUT_VIDEO, OUTPUT_VIDEO).context("failed to process video")?;

    let delta = start_time.elapsed();
    println!("Video processed successfully in {delta:?}: {OUTPUT_VIDEO}");

    Ok(())
}
