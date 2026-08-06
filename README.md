# Mimic

Mimic is a simple Linux utility for managing MIME type to application associations. It shows registered file types and lets you choose which application opens them by default. The app writes to `~/.config/mimeapps.list` through GIO, so `xdg-open` and desktop environments respect your choices.

Designed with minimalism and keyboard navigation in mind — a good fit for users of tiling window managers.

## Features

- Browse all registered MIME types with human-readable descriptions
- Filter by category and search by name
- Set the default application for any file type
- Hide rarely used types by default
- Keyboard shortcuts: `Ctrl+H` to focus search, `Esc` to clear it

## Dependencies

- GTK4
- GIO / GLib
- Rust toolchain (edition 2024)

## Building

```sh
cargo build --release
```

The binary will be at `target/release/mimic`.

## Installing (Arch Linux)

Build and install the package from `packaging/arch`. The source archive is
generated locally from the current git tag, so no download is needed.

```sh
cd packaging/arch
./make-src.sh          # creates mimic-0.1.0.tar.gz from tag v0.1.0
makepkg
sudo pacman -U mimic-0.1.0-1-x86_64.pkg.tar.zst
```

## Manual install

```sh
cargo build --release
install -Dm755 target/release/mimic ~/.local/bin/mimic
install -Dm644 data/org.mimic.Mimic.desktop ~/.local/share/applications/
install -Dm644 data/org.mimic.Mimic.svg ~/.local/share/icons/hicolor/scalable/apps/
```

## License

[MIT](LICENSE)
