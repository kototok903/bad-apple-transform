# Bad Apple Transform

## Idea

Bad Apple animation is purely black and white. This means, each frame can be represented as a set of pixel coords that flip on that frame. My idea: compute that set for each pixel, but make the pixels flip between a list of colors instead of just black and white (e.g., black -> white -> color 3 -> color 4 -> black -> ...) using the same flip set for each frame.

## Usage

1. Make sure you have [FFmpeg](https://www.ffmpeg.org/) CLI installed.
2. Run `cargo run --release`
   1. The input video will get downloaded automatically
   2. The output video will appear in `output/` on success
3. Edit the fields of `NCOLOR_CONFIG` in main.rs to get different results

## References

- https://github.com/nathanbabcock/ffmpeg-sidecar
  - https://github.com/nathanbabcock/ffmpeg-sidecar/blob/main/examples/h265_transcode.rs
- https://github.com/BloodyFish/bad-apple-manipulator
- https://docs.rs/anyhow/latest/anyhow/
