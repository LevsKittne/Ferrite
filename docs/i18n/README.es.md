# Ferrite — cargador de mods para juegos de Unity

[English](../../README.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [中文](README.zh-CN.md) · Español · [Português](README.pt-BR.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Türkçe](README.tr.md) · [Polski](README.pl.md) · [Italiano](README.it.md) · [Tiếng Việt](README.vi.md) · [Bahasa Indonesia](README.id.md)

Ferrite es un cargador de mods (mod loader) gratuito para cualquier juego hecho con Unity, tanto Mono como IL2CPP. El núcleo está escrito en Rust y se instala con un solo archivo `version.dll`. Es una alternativa a BepInEx y MelonLoader: arranca más rápido, casi no usa memoria y se instala con un clic.

## Funciones

- Cualquier juego de Unity, sin configuración por juego. Mono e IL2CPP, Unity 2020–6000.
- Los mods para Mono son bibliotecas C# normales con Harmony. Los mods para IL2CPP son bibliotecas en Rust (`ferrite-sdk`) con hooks y acceso a las clases del juego.
- Menú dentro del juego (F1): mods, ajustes, consola y opciones del cargador. Funciona con Direct3D 11, Direct3D 12, Vulkan y OpenGL.
- Los mods arrancan antes de la primera escena. Los mods y sus ajustes se recargan sin reiniciar el juego.
- Parcheadores de ensamblados con Mono.Cecil, dependencias con versiones, consola de comandos, depurador de Mono (dnSpy, Rider).
- Instalador en 12 idiomas.

## Instalación

1. Descarga `Ferrite-Setup.exe` desde [Releases](https://github.com/LevsKittne/Ferrite/releases).
2. Ponlo junto al `.exe` del juego y ejecútalo, o ejecútalo en cualquier sitio y elige la carpeta del juego.
3. Pon los mods en `Ferrite\mods`.
4. Inicia el juego. El menú se abre con F1.

Instalación silenciosa: `Ferrite-Setup.exe /silent "/dir=C:\ruta\al\juego"`.

## Desinstalación

Borra `version.dll` y la carpeta `Ferrite` de la carpeta del juego. Para iniciar el juego una vez sin mods, añade el parámetro `--no-mods`.

## Crear mods

- Juegos Mono: biblioteca C# `net472` que referencia `Ferrite.dll` y `0Harmony.dll`, ejemplo en `examples/HelloMod`.
- Juegos IL2CPP: crate de Rust `cdylib` con `ferrite-sdk`, ejemplo en `examples/hello-native`.

Detalles y código en el [README en inglés](../../README.md). Cambios por versión en el [CHANGELOG](../../CHANGELOG.md).

También se busca como: cargador de mods Unity, mod loader Unity, mods para juegos Unity, alternativa a BepInEx, alternativa a MelonLoader, mods IL2CPP, mods Mono, Harmony.
