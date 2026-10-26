# CP2077 Mod Launcher

[English](README.md) | [Türkçe](README.tr.md) | [Русский](README.ru.md) | [Español](README.es.md) | [Português (BR)](README.pt-BR.md) | [Français](README.fr.md) | [Polski](README.pl.md) | [Deutsch](README.de.md)

An unofficial community mod manager for Cyberpunk 2077 on Windows. Version 0.0.1 beta.

It is not made by or affiliated with CD PROJEKT RED, Nexus Mods or the NightCoop team.

## What it does

- Installs mods from .zip, .rar and .7z files. Drop a file on the window or press Install mod.
- Turns a mod on by copying its files into the game folder and off by removing exactly those files. Overwritten files are backed up and put back.
- Warns before a mod would overwrite a file that belongs to another mod.
- Finds mods you installed by hand and lets you import them.
- Keeps several versions of one mod in a single card, so you can see which one is current.
- Checks and installs NightCoop builds from the official website, and verifies each download with SHA-256.
- Browses and searches Nexus Mods with filters, tags and endless scrolling. Free accounts can install through nxm links or from the downloads folder.
- Shows game, mod and launcher logs, and clears them when you ask.
- Speaks English, Turkish, Russian, Spanish, Brazilian Portuguese, French, Polish and German.
- Has system, dark and light themes.

## Install

1. Download the zip from [Releases](https://github.com/This-null/cyberpunk-modloader/releases).
2. Unzip it and run the setup file.
3. Windows may show a SmartScreen warning because the installer is not code signed. Choose More info, then Run anyway.

## Privacy

Nothing is sent anywhere automatically. Bug reports are only saved on your computer. The launcher talks to the NightCoop website and Nexus Mods to list and download mods. Release notes go to Google Translate only if you allow it in Settings. Your Nexus API key stays in Windows Credential Manager on your computer.

## Build from source

You need Node 20 or newer, pnpm and stable Rust.

```
pnpm install
pnpm tauri dev
pnpm tauri build
```

## License

MIT, see [LICENSE](LICENSE). RAR files are unpacked with the UnRAR source through the unrar crate, which has its own license.

Use mods at your own risk and back up your saves first.
