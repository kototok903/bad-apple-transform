use std::fs::{File, rename};
use std::io::Write;
use std::path::Path;
use std::error::Error;
use std::thread;
use std::time::Duration;

use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::{FfmpegEvent, LogLevel};
use reqwest::blocking::get;

const INPUT_VIDEO: &str = "input/badapple.mov";
const OUTPUT_VIDEO: &str = "output/badapple.mp4";

const BAD_APPLE_VIDEO_URLS: &[&str] = &[
    "https://badapple.mov/badapple.mov",
    "https://web.archive.org/web/20260826212812/https://badapple.mov/badapple.mov",
];

const BLACK: [u8; 3] = [0x00, 0x00, 0x00];
const WHITE: [u8; 3] = [0xFF, 0xFF, 0xFF];
const TEAL:  [u8; 3] = [0x06, 0x94, 0x94];
const OUTPUT_COLORS: &[[u8; 3]] = &[BLACK, WHITE, TEAL];

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

#[derive(Clone)]
struct FrameInfo {
    black: bool,
    cnt: usize,
}

fn is_black(val: u8) -> bool {
    return val < 128
}

fn process_video(in_path_str: &str, out_path_str: &str) -> Result<(), Box<dyn Error>> {
    let mut ffmpeg_input = FfmpegCommand::new()
        .input(in_path_str)
        .format("rawvideo")
        .pix_fmt("gray")
        .pipe_stdout()
        .spawn()?;

    // extract metadata
    let mut frames_iter = ffmpeg_input.iter()?;
    let metadata = frames_iter.collect_metadata()?;
    let frames_iter = frames_iter.filter_frames();
    if metadata.output_streams.len() == 0 {
        return Err("no output_streams in input video metadata".into());
    }
    let Some(stream_data) = metadata.output_streams[0].video_data() else {
        return Err("no video data in input video metadata".into());
    };

    let (width, height) = (stream_data.width, stream_data.height);
    let fps = stream_data.fps;
    let duration_sec = metadata.duration().unwrap_or(0f64) as i32;
    let mut pixel_info = vec![FrameInfo { black: true, cnt: 0 }; (width * height) as usize];

    let mut ffmpeg_output = FfmpegCommand::new()
        .format("rawvideo")
        .pix_fmt("rgb24")
        .size(width, height)
        .rate(fps) // how do i use the same fps as input?
        .input("-")
        .codec_video("libx264")
        .pix_fmt("yuv420p") 
        .overwrite()
        .output(out_path_str)
        .spawn()?;
    let Some(mut output_stdin) = ffmpeg_output.take_stdin() else {
        return Err("failed to take output stdin".into());
    };
    let mut output_buffer = Vec::with_capacity((width * height * 3) as usize); // u8 to u24 pixels

    let logs_iter = ffmpeg_output.iter()?;
    let output_worker = thread::spawn(move || {
        let mut problems = Vec::new();
        for e in logs_iter {
            match e {
                FfmpegEvent::Log(LogLevel::Error | LogLevel::Fatal, e) => {
                    eprintln!("Output thread error: {e}");
                    problems.push(e);
                },
                FfmpegEvent::Progress(p) => println!("Progress: F{}\t{} / {:02}:{:02}:{:02}", p.frame, p.time, duration_sec / 3600, duration_sec / 60 % 60, duration_sec % 60),
                _ => {}
            }
        } 
        problems
    });

    let mut write_res = Ok(());
    for frame in frames_iter {
        println!("Frame {}, w: {}, h: {}", frame.frame_num, frame.width, frame.height);
        output_buffer.clear();
        for (info, val) in pixel_info.iter_mut().zip(frame.data) {
            if info.black != is_black(val) {
                info.black = !info.black;
                info.cnt = (info.cnt + 1) % OUTPUT_COLORS.len();
            }
            output_buffer.extend(OUTPUT_COLORS[info.cnt]);
        }
        write_res = output_stdin.write_all(&output_buffer);
        if write_res.is_err() {
            break;
        }
    }

    drop(output_stdin);
    let output_worker_res = output_worker.join();
    let output_status = ffmpeg_output.wait();
    let input_status = ffmpeg_input.wait();
    let problems = output_worker_res.map_err(|_| "output logs thread error")?;
    if problems.len() > 0 {
        // what should i do here with an list of strings?
    }
    let output_status = output_status?;
    if !output_status.success() {
        return Err(format!("Output thread exited with status {output_status}").into())
    }
    let input_status = input_status?;
    if !input_status.success() {
        return Err(format!("Output thread exited with status {output_status}").into())
    }
    write_res?;
    return Ok(());
}

fn main() {
    if let Err(e) = ensure_bad_apple_file(INPUT_VIDEO) {
        eprintln!("Failed to ensure Bad Apple video file: {e}");
        return;
    }
    if let Err(e) = process_video(INPUT_VIDEO, OUTPUT_VIDEO) {
        eprintln!("Failed to process video: {e}");
        return;
    }
    println!("Hello, world!");
}
