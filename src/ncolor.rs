use std::io::Write;
use std::path::Path;
use std::thread;

use anyhow::{Context, Result, anyhow, ensure};
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::{FfmpegEvent, LogLevel, OutputVideoFrame};

use crate::utils::Color;

#[derive(Clone, Copy)]
pub struct NColorPixelHeal {
    /// After this number of frames the pixel goes back to its original (B or W) color.
    pub frames: u32,

    /// This value decides whether pixels would slowly fade back to
    /// their original value, or switch immediatelly.
    pub fade: bool,
}

pub struct NColorConfig {
    /// Edit this to change what colors are used and in what order.
    /// Example: &\[BLACK, WHITE\] would give you the original video.
    pub palette: &'static [Color],

    /// Configure whether pixels go back to their original (B or W) color.
    /// Set to None to disable.
    pub pixel_heal: Option<NColorPixelHeal>,
}

#[derive(Clone)]
struct PixelState {
    is_black: bool,
    color_idx: usize,
    last_updated: u32,
}

impl PixelState {
    fn is_black(val: u8) -> bool {
        // TODO: maybe add different low and high thresholds
        val < 128
    }

    fn update(&mut self, config: &NColorConfig, new_val: u8, frame_num: u32) -> Color {
        if self.is_black != Self::is_black(new_val) {
            self.is_black = !self.is_black;
            self.color_idx = (self.color_idx + 1) % config.palette.len();
            self.last_updated = frame_num;
        }
        let mut color = config.palette[self.color_idx];
        if let Some(ph) = config.pixel_heal {
            let og_color_idx = if self.is_black { 0 } else { 1 }; // assuming BLACK is the first element in config.palette
            if frame_num > self.last_updated + ph.frames {
                self.color_idx = og_color_idx;
                color = config.palette[self.color_idx];
            } else if ph.fade {
                color = Color::blend(
                    config.palette[self.color_idx],
                    config.palette[og_color_idx],
                    (frame_num - self.last_updated) as f64 / ph.frames as f64,
                )
            }
        }
        color
    }
}

fn process_frames(
    config: &NColorConfig,
    frames_iter: impl Iterator<Item = OutputVideoFrame>,
    output: &mut impl Write,
    width: u32,
    height: u32,
) -> Result<()> {
    let mut pixel_states = vec![
        PixelState {
            is_black: true,
            color_idx: 0,
            last_updated: 0
        };
        (width * height) as usize
    ];
    let mut output_buffer = Vec::with_capacity((width * height * 3) as usize); // u8 to u24 pixels

    for frame in frames_iter {
        output_buffer.clear();
        for (ps, val) in pixel_states.iter_mut().zip(frame.data) {
            let color = ps.update(config, val, frame.frame_num);
            output_buffer.extend(color.bytes());
        }
        output.write_all(&output_buffer)?;
    }

    Ok(())
}

pub fn process_video(config: &NColorConfig, in_path: &Path, out_path: &Path) -> Result<()> {
    // create video decoder (input)
    let mut ffmpeg_decoder = FfmpegCommand::new()
        .input(in_path)
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
    let duration_sec = metadata.duration().unwrap_or(0.0) as i32;

    // create video encoder (output)
    let mut ffmpeg_encoder = FfmpegCommand::new()
        // video input (frames from stdin)
        .format("rawvideo")
        .pix_fmt("rgb24")
        .size(width, height)
        .rate(fps)
        .input("-")
        // audio input
        .input(in_path)
        // output
        .codec_audio("copy")
        .codec_video("libx264")
        .pix_fmt("yuv420p")
        .map("0:v")
        .map("1:a")
        .overwrite()
        .output(out_path)
        .spawn()
        .context("failed to spawn ffmpeg encoder")?;

    let mut encoder_stdin = ffmpeg_encoder
        .take_stdin()
        .context("failed to take output stdin")?;
    let encoder_logs_iter = ffmpeg_encoder.iter()?;

    // process encoder logs and print progress info
    let encoder_logs_worker = thread::spawn(move || {
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

    let processing_res = process_frames(config, frames_iter, &mut encoder_stdin, width, height);

    // release all resources
    drop(encoder_stdin);
    let encoder_logs_worker_res = encoder_logs_worker.join();
    let encoder_status = ffmpeg_encoder.wait();
    let decoder_status = ffmpeg_decoder.wait();

    // handle errors
    let problems = encoder_logs_worker_res.map_err(|_| anyhow!("output logs thread error"))?;
    let encoder_status = encoder_status?;
    ensure!(
        encoder_status.success(),
        "output thread exited with status {}, logged errors:\n{}",
        encoder_status,
        problems.join("\n")
    );
    let decoder_status = decoder_status?;
    ensure!(
        decoder_status.success(),
        "input thread exited with status {decoder_status}"
    );
    processing_res?;

    Ok(())
}
