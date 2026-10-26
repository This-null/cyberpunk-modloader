# CP2077 Mod Launcher

[English](README.md) | [Türkçe](README.tr.md) | [Русский](README.ru.md) | [Español](README.es.md) | [Português (BR)](README.pt-BR.md) | [Français](README.fr.md) | [Polski](README.pl.md) | [Deutsch](README.de.md)

Ein inoffizieller Mod-Manager aus der Community für Cyberpunk 2077 unter Windows. Version 0.0.1 Beta.

Er stammt nicht von CD PROJEKT RED, Nexus Mods oder dem NightCoop-Team und ist nicht mit ihnen verbunden.

## Was er kann

- Installiert Mods aus .zip-, .rar- und .7z-Dateien. Ziehe die Datei ins Fenster oder klicke auf Mod installieren.
- Schaltet eine Mod ein, indem er ihre Dateien in den Spieleordner kopiert, und aus, indem er genau diese Dateien löscht. Überschriebene Dateien werden gesichert und zurückgelegt.
- Warnt, bevor eine Mod die Datei einer anderen Mod überschreibt.
- Findet von Hand installierte Mods und lässt dich sie importieren.
- Fasst mehrere Versionen einer Mod in einer Karte zusammen, sodass man sieht, welche aktuell ist.
- Prüft und installiert NightCoop-Builds von der offiziellen Website und verifiziert jeden Download mit SHA-256.
- Durchsucht Nexus Mods mit Filtern, Tags und endlosem Scrollen. Kostenlose Konten können über nxm-Links oder den Download-Ordner installieren.
- Zeigt Logs von Spiel, Mods und Launcher und löscht sie auf Wunsch.
- Spricht Englisch, Türkisch, Russisch, Spanisch, brasilianisches Portugiesisch, Französisch, Polnisch und Deutsch.
- Hat System-, dunkles und helles Design.

## Installation

1. Lade das Zip von [Releases](https://github.com/This-null/cyberpunk-modloader/releases) herunter.
2. Entpacke es und starte das Setup.
3. Windows zeigt eventuell eine SmartScreen-Warnung, weil das Setup nicht signiert ist. Wähle Weitere Informationen und dann Trotzdem ausführen.

## Datenschutz

Nichts wird automatisch gesendet. Fehlerberichte werden nur auf deinem Computer gespeichert. Der Launcher spricht mit der NightCoop-Website und Nexus Mods, um Mods aufzulisten und herunterzuladen. Versionshinweise gehen nur an Google Übersetzer, wenn du es in den Einstellungen erlaubst. Dein Nexus-API-Schlüssel bleibt in der Windows-Anmeldeinformationsverwaltung auf deinem Computer.

## Aus dem Quellcode bauen

Du brauchst Node 20 oder neuer, pnpm und stabiles Rust.

```
pnpm install
pnpm tauri dev
pnpm tauri build
```

## Lizenz

MIT, siehe [LICENSE](LICENSE). RAR-Dateien werden mit dem UnRAR-Quellcode über das Paket unrar entpackt, das eine eigene Lizenz hat.

Nutze Mods auf eigene Gefahr und sichere vorher deine Spielstände.
