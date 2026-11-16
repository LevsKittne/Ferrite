# Ferrite — mod loader per i giochi Unity

[English](../../README.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [中文](README.zh-CN.md) · [Español](README.es.md) · [Português](README.pt-BR.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Türkçe](README.tr.md) · [Polski](README.pl.md) · Italiano · [Tiếng Việt](README.vi.md) · [Bahasa Indonesia](README.id.md)

Ferrite è un mod loader gratuito per qualsiasi gioco fatto con Unity, Mono e IL2CPP. Il nucleo è scritto in Rust e si installa con un solo file `version.dll`. È un'alternativa a BepInEx e MelonLoader: si avvia più in fretta, non occupa quasi memoria e si installa con un clic.

## Funzionalità

- Qualsiasi gioco Unity, senza configurazione per gioco. Mono e IL2CPP, Unity 2020–6000.
- Le mod per Mono sono normali librerie C# con Harmony. Le mod per IL2CPP sono librerie Rust (`ferrite-sdk`) con hook e accesso alle classi del gioco.
- Menu nel gioco (F1): mod, impostazioni, console, opzioni del loader. Funziona con Direct3D 11, Direct3D 12, Vulkan e OpenGL.
- Le mod partono prima della prima scena. Mod e impostazioni si ricaricano senza riavviare il gioco.
- Patcher di assembly con Mono.Cecil, dipendenze con versioni, console dei comandi, debugger Mono (dnSpy, Rider).
- Installer in 12 lingue.

## Installazione

1. Scarica `Ferrite-Setup.exe` da [Releases](https://github.com/LevsKittne/Ferrite/releases).
2. Mettilo accanto al `.exe` del gioco ed eseguilo, oppure eseguilo ovunque e scegli la cartella del gioco.
3. Metti le mod in `Ferrite\mods`.
4. Avvia il gioco. Il menu si apre con F1.

Installazione silenziosa: `Ferrite-Setup.exe /silent "/dir=C:\percorso\del\gioco"`.

## Disinstallazione

Elimina `version.dll` e la cartella `Ferrite` dalla cartella del gioco. Per avviare il gioco una volta senza mod, aggiungi il parametro `--no-mods`.

## Creare mod

- Giochi Mono: libreria C# `net472` che fa riferimento a `Ferrite.dll` e `0Harmony.dll`, esempio in `examples/HelloMod`.
- Giochi IL2CPP: crate Rust `cdylib` con `ferrite-sdk`, esempio in `examples/hello-native`.

Dettagli e codice nel [README in inglese](../../README.md). Modifiche per versione nel [CHANGELOG](../../CHANGELOG.md).

Cercato anche come: mod loader Unity, caricatore di mod Unity, mod per giochi Unity, alternativa a BepInEx, alternativa a MelonLoader, mod IL2CPP, mod Mono, Harmony.
