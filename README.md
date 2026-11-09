# Bad Apple Transform

## Idea

The Bad Apple is purely black and white. This means, each frame can be represented as a set of pixel coords that flip on that frame. My idea: compute that set for each pixel, but make the pixels flip between a list of colors instead of just black and white (e.g., black -> white -> color 3 -> black -> ...) using the same flip set for each frame.

## References

- https://github.com/nathanbabcock/ffmpeg-sidecar
- https://github.com/BloodyFish/bad-apple-manipulator
