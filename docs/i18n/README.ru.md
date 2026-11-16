# Ferrite — загрузчик модов для игр на Unity

[English](../../README.md) · Русский · [Українська](README.uk.md) · [中文](README.zh-CN.md) · [Español](README.es.md) · [Português](README.pt-BR.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Türkçe](README.tr.md) · [Polski](README.pl.md) · [Italiano](README.it.md) · [Tiếng Việt](README.vi.md) · [Bahasa Indonesia](README.id.md)

Ferrite — бесплатный загрузчик модов для любых игр на Unity, и Mono, и IL2CPP. Ядро написано на Rust, ставится одним файлом `version.dll`. Альтернатива BepInEx и MelonLoader: быстрее запускается, почти не ест память, ставится в один клик.

## Возможности

- Любая игра на Unity, без настройки под конкретную игру. Mono и IL2CPP, Unity 2020–6000.
- Моды для Mono — обычные библиотеки C# с Harmony. Моды для IL2CPP — библиотеки на Rust (`ferrite-sdk`) с хуками и доступом к классам игры.
- Меню в игре (F1): моды, настройки, консоль, параметры загрузчика. Работает на Direct3D 11, Direct3D 12, Vulkan и OpenGL.
- Моды запускаются до первой сцены. Моды и их настройки перезагружаются без перезапуска игры.
- Патчеры сборок на Mono.Cecil, зависимости с версиями, консоль с командами, отладчик Mono (dnSpy, Rider).
- Установщик на 12 языках.

## Установка

1. Скачайте `Ferrite-Setup.exe` со страницы [Releases](https://github.com/LevsKittne/Ferrite/releases).
2. Положите его рядом с `.exe` игры и запустите, или запустите где угодно и выберите папку игры.
3. Положите моды в `Ferrite\mods`.
4. Запустите игру. Меню открывается на F1.

Тихая установка: `Ferrite-Setup.exe /silent "/dir=C:\путь\к\игре"`.

## Удаление

Удалите `version.dll` и папку `Ferrite` из папки игры. Чтобы один раз запустить игру без модов, добавьте параметр `--no-mods`.

## Написание модов

- Mono-игры: библиотека C# `net472` со ссылками на `Ferrite.dll` и `0Harmony.dll`, пример — `examples/HelloMod`.
- IL2CPP-игры: крейт Rust `cdylib` на `ferrite-sdk`, пример — `examples/hello-native`.

Подробности и код — в [README на английском](../../README.md). Изменения по версиям — в [CHANGELOG](../../CHANGELOG.md).

Ищут как: загрузчик модов Unity, мод лоадер Unity, моды для Unity игр, BepInEx альтернатива, MelonLoader альтернатива, Unity mod loader, IL2CPP моды, Mono моды, Harmony.
