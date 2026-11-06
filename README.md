# RepoBell

A small tray app for Ubuntu that tells you when your team changes the Git repos on your computer.

RepoBell is in early development and not usable yet.

## What it will do

- Find the repos in your project folders.
- Show what's new on the main branch since you last looked: who pushed and what changed.
- Send one desktop notification per check, never one per commit.
- Update a repo with one click. If an update would cause a conflict, it stops and changes nothing.
- Use plain words, so designers can use it as well as developers.

## Build

You need Rust and the Tauri system packages (Ubuntu 22.04):

```sh
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
cargo test
```

## Docs

- [Roadmap](ROADMAP.md)
- [Decisions](docs/decisions.md)
- [Mockups](docs/mockups)

## License

[MIT](LICENSE)
