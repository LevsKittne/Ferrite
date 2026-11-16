# Ferrite — loader modów do gier na Unity

[English](../../README.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [中文](README.zh-CN.md) · [Español](README.es.md) · [Português](README.pt-BR.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Türkçe](README.tr.md) · Polski · [Italiano](README.it.md) · [Tiếng Việt](README.vi.md) · [Bahasa Indonesia](README.id.md)

Ferrite to darmowy loader modów (mod loader) do każdej gry na Unity, Mono i IL2CPP. Rdzeń jest napisany w Rust i instaluje się jednym plikiem `version.dll`. Alternatywa dla BepInEx i MelonLoader: szybciej startuje, prawie nie zużywa pamięci, instaluje się jednym kliknięciem.

## Funkcje

- Każda gra na Unity, bez konfiguracji pod konkretną grę. Mono i IL2CPP, Unity 2020–6000.
- Mody do Mono to zwykłe biblioteki C# z Harmony. Mody do IL2CPP to biblioteki w Rust (`ferrite-sdk`) z hookami i dostępem do klas gry.
- Menu w grze (F1): mody, ustawienia, konsola, opcje loadera. Działa z Direct3D 11, Direct3D 12, Vulkan i OpenGL.
- Mody startują przed pierwszą sceną. Mody i ich ustawienia przeładowują się bez restartu gry.
- Patchery assembly na Mono.Cecil, zależności z wersjami, konsola poleceń, debugger Mono (dnSpy, Rider).
- Instalator w 12 językach.

## Instalacja

1. Pobierz `Ferrite-Setup.exe` ze strony [Releases](https://github.com/LevsKittne/Ferrite/releases).
2. Umieść go obok pliku `.exe` gry i uruchom, albo uruchom gdziekolwiek i wybierz folder gry.
3. Wrzuć mody do `Ferrite\mods`.
4. Uruchom grę. Menu otwiera się klawiszem F1.

Cicha instalacja: `Ferrite-Setup.exe /silent "/dir=C:\ścieżka\do\gry"`.

## Odinstalowanie

Usuń `version.dll` i folder `Ferrite` z folderu gry. Aby raz uruchomić grę bez modów, dodaj parametr `--no-mods`.

## Tworzenie modów

- Gry Mono: biblioteka C# `net472` z referencjami do `Ferrite.dll` i `0Harmony.dll`, przykład w `examples/HelloMod`.
- Gry IL2CPP: crate Rust `cdylib` z `ferrite-sdk`, przykład w `examples/hello-native`.

Szczegóły i kod w [README po angielsku](../../README.md). Zmiany w wersjach w [CHANGELOG](../../CHANGELOG.md).

Szukane też jako: loader modów Unity, mod loader Unity, mody do gier Unity, alternatywa dla BepInEx, alternatywa dla MelonLoader, mody IL2CPP, mody Mono, Harmony.
