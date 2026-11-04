# Privacy

CP2077 Mod Launcher runs on your computer. It has no accounts, no analytics and no tracking.

## What stays on your computer

Your mod library, settings, logs and your Nexus Mods API key. The key is kept in the Windows credential store, not in a plain file. Error reports are only written when you ask for one.

## What leaves your computer

Only when you use the feature that needs it:

- **Nexus Mods.** With your own key, the launcher searches, checks for updates and downloads. Nexus Mods' own privacy policy applies to that traffic.
- **Updates.** The launcher asks GitHub for the latest release of this project.
- **NightCoop.** The release list and downloads come from the NightCoop project.
- **Profile codes.** If you press share online, the mod list of that profile (Nexus ids, versions, names, order and on/off state) goes to a small server run by this project. It is kept for 12 hours and then deleted. It is tied to a random install id, not to your name. You can reset that id in the settings.
- **Link sharing.** The pack goes straight from your computer to your friend through a Cloudflare tunnel. Cloudflare relays the traffic like any proxy, and nothing is stored. The first time you use it, the launcher downloads the open source `cloudflared` tool from its official GitHub releases and checks it against a fixed checksum.
- **Translation.** If you turn on translation of mod notes, the note text is sent to a public Google Translate endpoint.
- **Discord Rich Presence.** The launcher only talks to the Discord app on your computer. It shows that you are managing mods or playing, the number of enabled mods and the name of the Nexus page you are on. You can switch it off in the settings.

## Questions

Open an issue at https://github.com/This-null/cyberpunk-modloader/issues.
