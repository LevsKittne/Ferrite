# Ferrite — Mod-Loader für Unity-Spiele

[English](../../README.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [中文](README.zh-CN.md) · [Español](README.es.md) · [Português](README.pt-BR.md) · Deutsch · [Français](README.fr.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Türkçe](README.tr.md) · [Polski](README.pl.md) · [Italiano](README.it.md) · [Tiếng Việt](README.vi.md) · [Bahasa Indonesia](README.id.md)

Ferrite ist ein kostenloser Mod-Loader für jedes Unity-Spiel, Mono und IL2CPP. Der Kern ist in Rust geschrieben und wird als eine einzige Datei `version.dll` installiert. Eine Alternative zu BepInEx und MelonLoader: startet schneller, braucht kaum Speicher, Installation mit einem Klick.

## Funktionen

- Jedes Unity-Spiel, ohne Einrichtung pro Spiel. Mono und IL2CPP, Unity 2020–6000.
- Mods für Mono sind normale C#-Bibliotheken mit Harmony. Mods für IL2CPP sind Rust-Bibliotheken (`ferrite-sdk`) mit Hooks und Zugriff auf die Klassen des Spiels.
- Menü im Spiel (F1): Mods, Einstellungen, Konsole, Loader-Optionen. Funktioniert mit Direct3D 11, Direct3D 12, Vulkan und OpenGL.
- Mods starten vor der ersten Szene. Mods und ihre Einstellungen werden ohne Neustart des Spiels neu geladen.
- Assembly-Patcher mit Mono.Cecil, Abhängigkeiten mit Versionen, Befehlskonsole, Mono-Debugger (dnSpy, Rider).
- Installer in 12 Sprachen.

## Installation

1. `Ferrite-Setup.exe` von [Releases](https://github.com/LevsKittne/Ferrite/releases) herunterladen.
2. Neben die `.exe` des Spiels legen und starten, oder irgendwo starten und den Spielordner wählen.
3. Mods in `Ferrite\mods` legen.
4. Spiel starten. Das Menü öffnet sich mit F1.

Stille Installation: `Ferrite-Setup.exe /silent "/dir=C:\Pfad\zum\Spiel"`.

## Deinstallation

`version.dll` und den Ordner `Ferrite` aus dem Spielordner löschen. Um das Spiel einmal ohne Mods zu starten, den Parameter `--no-mods` anhängen.

## Mods schreiben

- Mono-Spiele: C#-Bibliothek `net472` mit Verweisen auf `Ferrite.dll` und `0Harmony.dll`, Beispiel in `examples/HelloMod`.
- IL2CPP-Spiele: Rust-Crate `cdylib` mit `ferrite-sdk`, Beispiel in `examples/hello-native`.

Details und Code im [englischen README](../../README.md). Änderungen pro Version im [CHANGELOG](../../CHANGELOG.md).

Auch gesucht als: Unity Mod Loader, Mods für Unity-Spiele, BepInEx Alternative, MelonLoader Alternative, IL2CPP Mods, Mono Mods, Harmony.
