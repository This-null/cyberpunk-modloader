# CP2077 Mod Launcher

[English](README.md) | [Türkçe](README.tr.md) | [Русский](README.ru.md) | [Español](README.es.md) | [Português (BR)](README.pt-BR.md) | [Français](README.fr.md) | [Polski](README.pl.md) | [Deutsch](README.de.md)

An unofficial community mod manager for Cyberpunk 2077 on Windows. Version 0.0.6 beta.

It is not made by or affiliated with CD PROJEKT RED, Nexus Mods or the NightCoop team.

## What it does

- Installs mods from .zip, .rar and .7z files. Drop a file on the window or press Install mod.
- Turns a mod on by copying its files into the game folder and off by removing exactly those files. Overwritten files are backed up and put back.
- Warns before a mod would overwrite a file that belongs to another mod.
- Finds mods you installed by hand and lets you import them.
- Keeps several versions of one mod in a single card, so you can see which one is current.
- Checks and installs NightCoop builds from the official website, and verifies each download with SHA-256.
- Browses and searches Nexus Mods with filters, tags and endless scrolling. Free accounts can install through nxm links or from the downloads folder.
- Shows what each Nexus mod requires, and shares your mod list as a profile (a code or a file) so others can install what they are missing.
- Shows game, mod and launcher logs, and clears them when you ask.
- Speaks English, Turkish, Russian, Spanish, Brazilian Portuguese, French, Polish and German.
- Has system, dark and light themes.
- Installs FOMOD mods with their option dialog, checks CET, RED4ext, ArchiveXL and other frameworks, and updates mods you own as Premium. Questions or ideas? Join our [Discord](https://discord.gg/EaNcFyCEqP).
- Shows which mod replaced which mod's files, keeps one previous version of an updated mod so you can roll back, filters the mod list, groups mods under your own headers, and starts the game from the sidebar.

## Install

1. Download the zip from [Releases](https://github.com/This-null/cyberpunk-modloader/releases).
2. Unzip it and run the setup file.
3. Windows may show a SmartScreen warning because the installer is not code signed. Choose More info, then Run anyway.

## Privacy

Nothing is sent anywhere automatically. Bug reports are only saved on your computer. The launcher talks to the NightCoop website and Nexus Mods to list and download mods. It also reads the latest version number from GitHub to tell you about updates. You can turn this off in Settings. Release notes and mod descriptions go to Google Translate only if you allow it in Settings. Sharing a profile online sends only your mod list (sources and versions, no files) and a random ID made on your PC to our server, only when you press the button, and the code expires after 12 hours. Your Nexus API key stays in Windows Credential Manager on your computer.

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
