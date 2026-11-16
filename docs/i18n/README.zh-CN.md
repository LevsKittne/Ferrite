# Ferrite — Unity 游戏模组加载器

[English](../../README.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · 中文 · [Español](README.es.md) · [Português](README.pt-BR.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Türkçe](README.tr.md) · [Polski](README.pl.md) · [Italiano](README.it.md) · [Tiếng Việt](README.vi.md) · [Bahasa Indonesia](README.id.md)

Ferrite 是一个免费的 Unity 游戏模组加载器（Mod 加载器、MOD 框架），同时支持 Mono 和 IL2CPP。核心用 Rust 编写，只需一个 `version.dll` 文件即可安装。它是 BepInEx 和 MelonLoader 的替代品：启动更快，几乎不占额外内存，一键安装。

## 功能

- 支持任何 Unity 游戏，无需针对单个游戏配置。支持 Mono 和 IL2CPP，Unity 2020–6000。
- Mono 游戏的模组是普通的 C# 类库，可使用 Harmony。IL2CPP 游戏的模组是 Rust 库（`ferrite-sdk`），支持函数钩子（Hook）和访问游戏类。
- 游戏内菜单（F1）：模组列表、设置、控制台、加载器选项。支持 Direct3D 11、Direct3D 12、Vulkan 和 OpenGL。
- 模组在第一个场景之前启动。模组及其设置可以在不重启游戏的情况下热重载。
- 基于 Mono.Cecil 的程序集补丁（Patcher）、带版本的依赖、命令控制台、Mono 调试器（dnSpy、Rider）。
- 安装程序支持 12 种语言（包括中文）。

## 安装

1. 从 [Releases](https://github.com/LevsKittne/Ferrite/releases) 页面下载 `Ferrite-Setup.exe`。
2. 把它放到游戏的 `.exe` 旁边并运行，或者在任意位置运行并选择游戏文件夹。
3. 把模组放进 `Ferrite\mods`。
4. 启动游戏，按 F1 打开菜单。

静默安装：`Ferrite-Setup.exe /silent "/dir=C:\游戏\路径"`。

## 卸载

从游戏文件夹中删除 `version.dll` 和 `Ferrite` 文件夹。若只想不加载模组运行一次，启动参数加 `--no-mods`。

## 编写模组

- Mono 游戏：`net472` C# 类库，引用 `Ferrite.dll` 和 `0Harmony.dll`，示例见 `examples/HelloMod`。
- IL2CPP 游戏：使用 `ferrite-sdk` 的 Rust `cdylib` 包，示例见 `examples/hello-native`。

详细说明和代码见[英文 README](../../README.md)，版本变更见 [CHANGELOG](../../CHANGELOG.md)。

搜索关键词：Unity 模组加载器、Unity Mod 加载器、Unity MOD 框架、BepInEx 替代品、MelonLoader 替代品、IL2CPP 模组、Mono 模组、Unity mod loader、Harmony。
