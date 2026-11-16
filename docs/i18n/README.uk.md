# Ferrite — завантажувач модів для ігор на Unity

[English](../../README.md) · [Русский](README.ru.md) · Українська · [中文](README.zh-CN.md) · [Español](README.es.md) · [Português](README.pt-BR.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Türkçe](README.tr.md) · [Polski](README.pl.md) · [Italiano](README.it.md) · [Tiếng Việt](README.vi.md) · [Bahasa Indonesia](README.id.md)

Ferrite — безкоштовний завантажувач модів для будь-яких ігор на Unity, і Mono, і IL2CPP. Ядро написане на Rust, встановлюється одним файлом `version.dll`. Альтернатива BepInEx і MelonLoader: швидше запускається, майже не їсть пам'ять, встановлюється в один клік.

## Можливості

- Будь-яка гра на Unity, без налаштування під конкретну гру. Mono та IL2CPP, Unity 2020–6000.
- Моди для Mono — звичайні бібліотеки C# з Harmony. Моди для IL2CPP — бібліотеки на Rust (`ferrite-sdk`) з хуками та доступом до класів гри.
- Меню в грі (F1): моди, налаштування, консоль, параметри завантажувача. Працює на Direct3D 11, Direct3D 12, Vulkan та OpenGL.
- Моди запускаються до першої сцени. Моди та їхні налаштування перезавантажуються без перезапуску гри.
- Патчери збірок на Mono.Cecil, залежності з версіями, консоль з командами, відлагоджувач Mono (dnSpy, Rider).
- Інсталятор 12 мовами.

## Встановлення

1. Завантажте `Ferrite-Setup.exe` зі сторінки [Releases](https://github.com/LevsKittne/Ferrite/releases).
2. Покладіть його поруч із `.exe` гри та запустіть, або запустіть будь-де та виберіть папку гри.
3. Покладіть моди в `Ferrite\mods`.
4. Запустіть гру. Меню відкривається на F1.

Тихе встановлення: `Ferrite-Setup.exe /silent "/dir=C:\шлях\до\гри"`.

## Видалення

Видаліть `version.dll` і папку `Ferrite` з папки гри. Щоб один раз запустити гру без модів, додайте параметр `--no-mods`.

## Написання модів

- Mono-ігри: бібліотека C# `net472` з посиланнями на `Ferrite.dll` і `0Harmony.dll`, приклад — `examples/HelloMod`.
- IL2CPP-ігри: крейт Rust `cdylib` на `ferrite-sdk`, приклад — `examples/hello-native`.

Подробиці та код — у [README англійською](../../README.md). Зміни за версіями — у [CHANGELOG](../../CHANGELOG.md).

Шукають як: завантажувач модів Unity, мод лоадер Unity, моди для ігор Unity, альтернатива BepInEx, альтернатива MelonLoader, Unity mod loader, IL2CPP моди, Mono моди, Harmony.
