# CP2077 Mod Launcher

[English](README.md) | [Türkçe](README.tr.md) | [Русский](README.ru.md) | [Español](README.es.md) | [Português (BR)](README.pt-BR.md) | [Français](README.fr.md) | [Polski](README.pl.md) | [Deutsch](README.de.md)

Un gestionnaire de mods non officiel, fait par la communauté, pour Cyberpunk 2077 sur Windows. Version 0.0.6 bêta.

Il n'est pas créé par CD PROJEKT RED, Nexus Mods ou l'équipe NightCoop, ni affilié à eux.

## Ce qu'il fait

- Installe des mods depuis des fichiers .zip, .rar et .7z. Déposez le fichier sur la fenêtre ou cliquez sur Installer un mod.
- Active un mod en copiant ses fichiers dans le dossier du jeu et le désactive en supprimant exactement ces fichiers. Les fichiers écrasés sont sauvegardés puis remis.
- Prévient avant qu'un mod n'écrase un fichier d'un autre mod.
- Trouve les mods installés à la main et vous laisse les importer.
- Regroupe plusieurs versions d'un même mod dans une seule carte, pour voir laquelle est la bonne.
- Vérifie et installe les versions de NightCoop depuis le site officiel, et contrôle chaque téléchargement avec SHA-256.
- Parcourt et recherche Nexus Mods avec filtres, tags et défilement infini. Les comptes gratuits peuvent installer via les liens nxm ou depuis le dossier de téléchargements.
- Montre ce que requiert chaque mod Nexus et partage votre liste de mods comme profil (un code ou un fichier) pour que d'autres installent ce qui leur manque.
- Affiche les journaux du jeu, des mods et du launcher, et les efface sur demande.
- Parle anglais, turc, russe, espagnol, portugais du Brésil, français, polonais et allemand.
- Propose des thèmes système, sombre et clair.
- Installe les mods FOMOD avec leur fenêtre d'options, vérifie CET, RED4ext, ArchiveXL et d'autres frameworks, et met à jour les mods avec Premium. Une question ou une idée ? Rejoins notre [Discord](https://discord.gg/EaNcFyCEqP).
- Montre quel mod a remplacé les fichiers de quel mod, garde une version précédente d'un mod mis à jour pour revenir en arrière, filtre la liste des mods, regroupe les mods sous tes propres titres et lance le jeu depuis la barre latérale.

## Installation

1. Téléchargez le zip depuis [Releases](https://github.com/This-null/cyberpunk-modloader/releases).
2. Décompressez-le et lancez l'installateur.
3. Windows peut afficher un avertissement SmartScreen car l'installateur n'est pas signé. Choisissez Informations complémentaires, puis Exécuter quand même.

## Confidentialité

Rien n'est envoyé automatiquement. Les rapports d'erreur sont seulement enregistrés sur votre ordinateur. Le launcher contacte le site de NightCoop et Nexus Mods pour lister et télécharger les mods. Il lit aussi le numéro de la dernière version sur GitHub pour signaler les mises à jour. Vous pouvez le désactiver dans les réglages. Les notes de version et les descriptions des mods vont à Google Traduction seulement si vous l'autorisez dans les réglages. Partager un profil en ligne envoie à notre serveur uniquement votre liste de mods (sources et versions, sans fichiers) et un identifiant aléatoire créé sur votre ordinateur, seulement quand vous appuyez sur le bouton, et le code expire après 12 heures. Votre clé d'API Nexus reste dans le Gestionnaire d'informations d'identification Windows de votre ordinateur.

## Compiler depuis les sources

Il faut Node 20 ou plus récent, pnpm et Rust stable.

```
pnpm install
pnpm tauri dev
pnpm tauri build
```

## Licence

MIT, voir [LICENSE](LICENSE). Les fichiers RAR sont décompressés avec le code d'UnRAR via le paquet unrar, qui a sa propre licence.

Utilisez les mods à vos risques et sauvegardez vos parties avant.
