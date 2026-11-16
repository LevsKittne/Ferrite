# Ferrite — Unity 게임용 모드 로더

[English](../../README.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [中文](README.zh-CN.md) · [Español](README.es.md) · [Português](README.pt-BR.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [日本語](README.ja.md) · 한국어 · [Türkçe](README.tr.md) · [Polski](README.pl.md) · [Italiano](README.it.md) · [Tiếng Việt](README.vi.md) · [Bahasa Indonesia](README.id.md)

Ferrite는 Unity로 만든 모든 게임(Mono와 IL2CPP 모두)을 위한 무료 모드 로더입니다. 핵심은 Rust로 작성되었고 `version.dll` 파일 하나로 설치됩니다. BepInEx와 MelonLoader의 대안으로, 더 빨리 시작하고 메모리를 거의 쓰지 않으며 클릭 한 번으로 설치됩니다.

## 기능

- 게임별 설정 없이 모든 Unity 게임에서 동작. Mono와 IL2CPP, Unity 2020–6000.
- Mono용 모드는 Harmony를 쓰는 일반 C# 라이브러리입니다. IL2CPP용 모드는 Rust 라이브러리(`ferrite-sdk`)로, 후킹과 게임 클래스 접근을 지원합니다.
- 게임 내 메뉴(F1): 모드, 설정, 콘솔, 로더 옵션. Direct3D 11, Direct3D 12, Vulkan, OpenGL 지원.
- 모드는 첫 씬보다 먼저 시작됩니다. 모드와 설정은 게임을 재시작하지 않고 다시 불러올 수 있습니다.
- Mono.Cecil 기반 어셈블리 패처, 버전이 있는 의존성, 명령 콘솔, Mono 디버거(dnSpy, Rider).
- 12개 언어 설치 프로그램(한국어 포함).

## 설치

1. [Releases](https://github.com/LevsKittne/Ferrite/releases)에서 `Ferrite-Setup.exe`를 내려받습니다.
2. 게임 `.exe` 옆에 두고 실행하거나, 아무 곳에서 실행한 뒤 게임 폴더를 선택합니다.
3. 모드를 `Ferrite\mods`에 넣습니다.
4. 게임을 실행합니다. F1로 메뉴가 열립니다.

조용한 설치: `Ferrite-Setup.exe /silent "/dir=C:\게임\경로"`.

## 제거

게임 폴더에서 `version.dll`과 `Ferrite` 폴더를 삭제합니다. 한 번만 모드 없이 실행하려면 실행 옵션에 `--no-mods`를 추가합니다.

## 모드 만들기

- Mono 게임: `Ferrite.dll`과 `0Harmony.dll`을 참조하는 `net472` C# 라이브러리, 예제는 `examples/HelloMod`.
- IL2CPP 게임: `ferrite-sdk`를 쓰는 Rust `cdylib` 크레이트, 예제는 `examples/hello-native`.

자세한 내용과 코드는 [영어 README](../../README.md), 버전별 변경 사항은 [CHANGELOG](../../CHANGELOG.md)를 보세요.

검색 키워드: Unity 모드 로더, 유니티 모드 적용, Unity 게임 모드, BepInEx 대안, MelonLoader 대안, IL2CPP 모드, Mono 모드, Unity mod loader, Harmony.
