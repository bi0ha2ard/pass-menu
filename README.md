# pass-menu

A simple fuzzy-finder UI for `pass`, `ssh`, and `man`.

Built with [`winit`](https://github.com/rust-windowing/winit) and software-rendered with [`softbuffer`](https://github.com/rust-windowing/softbuffer).
Fuzzy matching provided by [`frizbee`](https://github.com/saghen/frizbee), font rendering by [`ab_glyph`](https://github.com/alexheretic/ab-glyph)

## Runtime dependencies

- `xdg-open` for URL opening
- `notify-send` for notifications

### X11
- `xdotool`
- `xclip`

### Wayland
- `wtype`
- `wl-copy`

### ssh/man
- A terminal emulator (customize by setting `CUSTOM_TERMINAL="alacritty"` during build)
- `nvim` as the `MANPAGER`

## Build options

Use the `x11` or `wayland` features to compile for the respective backends.

The font specified by the `CUSTOM_FONT_PATH` environment variable will be baked into the binary and used for all text rendering.

For all other config changes (font sizes, colors), fork and modify the source.

## Installation

```bash
# X11
CUSTOM_FONT_PATH=/usr/share/fonts/truetype/DejaVuSans.ttf cargo install --path . --features x11

# wayland
CUSTOM_FONT_PATH=/usr/share/fonts/truetype/DejaVuSans.ttf cargo install --path . --features wayland
```

## Usage

`pass` operations:

```bash
pass-menu <subcommand>

# Show commands
pass-menu help
```

`ssh` operations:

```
ssh-menu ssh   # ssh session
ssh-menu sftp  # sftp session
```

`man` operations:

```
man-menu
```
