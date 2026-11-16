# Ferrite — chargeur de mods pour les jeux Unity

[English](../../README.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [中文](README.zh-CN.md) · [Español](README.es.md) · [Português](README.pt-BR.md) · [Deutsch](README.de.md) · Français · [日本語](README.ja.md) · [한국어](README.ko.md) · [Türkçe](README.tr.md) · [Polski](README.pl.md) · [Italiano](README.it.md) · [Tiếng Việt](README.vi.md) · [Bahasa Indonesia](README.id.md)

Ferrite est un chargeur de mods (mod loader) gratuit pour tous les jeux Unity, Mono et IL2CPP. Le cœur est écrit en Rust et s'installe avec un seul fichier `version.dll`. C'est une alternative à BepInEx et MelonLoader : démarrage plus rapide, presque pas de mémoire en plus, installation en un clic.

## Fonctionnalités

- N'importe quel jeu Unity, sans configuration par jeu. Mono et IL2CPP, Unity 2020–6000.
- Les mods pour Mono sont de simples bibliothèques C# avec Harmony. Les mods pour IL2CPP sont des bibliothèques Rust (`ferrite-sdk`) avec hooks et accès aux classes du jeu.
- Menu en jeu (F1) : mods, réglages, console, options du chargeur. Fonctionne avec Direct3D 11, Direct3D 12, Vulkan et OpenGL.
- Les mods démarrent avant la première scène. Les mods et leurs réglages se rechargent sans redémarrer le jeu.
- Patchers d'assemblies avec Mono.Cecil, dépendances avec versions, console de commandes, débogueur Mono (dnSpy, Rider).
- Installateur en 12 langues.

## Installation

1. Téléchargez `Ferrite-Setup.exe` depuis [Releases](https://github.com/LevsKittne/Ferrite/releases).
2. Placez-le à côté du `.exe` du jeu et lancez-le, ou lancez-le n'importe où et choisissez le dossier du jeu.
3. Mettez les mods dans `Ferrite\mods`.
4. Lancez le jeu. Le menu s'ouvre avec F1.

Installation silencieuse : `Ferrite-Setup.exe /silent "/dir=C:\chemin\du\jeu"`.

## Désinstallation

Supprimez `version.dll` et le dossier `Ferrite` du dossier du jeu. Pour lancer le jeu une fois sans mods, ajoutez le paramètre `--no-mods`.

## Créer des mods

- Jeux Mono : bibliothèque C# `net472` qui référence `Ferrite.dll` et `0Harmony.dll`, exemple dans `examples/HelloMod`.
- Jeux IL2CPP : crate Rust `cdylib` avec `ferrite-sdk`, exemple dans `examples/hello-native`.

Détails et code dans le [README en anglais](../../README.md). Changements par version dans le [CHANGELOG](../../CHANGELOG.md).

Aussi recherché comme : chargeur de mods Unity, mod loader Unity, mods pour jeux Unity, alternative à BepInEx, alternative à MelonLoader, mods IL2CPP, mods Mono, Harmony.
