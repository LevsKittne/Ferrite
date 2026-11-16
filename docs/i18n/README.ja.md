# Ferrite — Unity ゲーム用 MOD ローダー

[English](../../README.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [中文](README.zh-CN.md) · [Español](README.es.md) · [Português](README.pt-BR.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · 日本語 · [한국어](README.ko.md) · [Türkçe](README.tr.md) · [Polski](README.pl.md) · [Italiano](README.it.md) · [Tiếng Việt](README.vi.md) · [Bahasa Indonesia](README.id.md)

Ferrite は、Unity で作られたあらゆるゲーム（Mono と IL2CPP の両方）に対応した無料の MOD ローダーです。コアは Rust で書かれており、`version.dll` ひとつでインストールできます。BepInEx や MelonLoader の代わりとして使え、起動が速く、メモリをほとんど使わず、ワンクリックで導入できます。

## 機能

- ゲームごとの設定なしで、どの Unity ゲームでも動作。Mono と IL2CPP、Unity 2020〜6000。
- Mono 用 MOD は Harmony を使える普通の C# ライブラリ。IL2CPP 用 MOD は Rust ライブラリ（`ferrite-sdk`）で、フックやゲームのクラスにアクセスできます。
- ゲーム内メニュー（F1）：MOD 一覧、設定、コンソール、ローダーのオプション。Direct3D 11、Direct3D 12、Vulkan、OpenGL に対応。
- MOD は最初のシーンより前に起動。MOD と設定はゲームを再起動せずにリロード可能。
- Mono.Cecil によるアセンブリパッチャー、バージョン付き依存関係、コマンドコンソール、Mono デバッガー（dnSpy、Rider）。
- 12 言語対応のインストーラー（日本語あり）。

## インストール

1. [Releases](https://github.com/LevsKittne/Ferrite/releases) から `Ferrite-Setup.exe` をダウンロード。
2. ゲームの `.exe` と同じフォルダーに置いて実行するか、任意の場所で実行してゲームのフォルダーを選択。
3. MOD を `Ferrite\mods` に入れる。
4. ゲームを起動。F1 でメニューが開きます。

サイレントインストール：`Ferrite-Setup.exe /silent "/dir=C:\ゲーム\のパス"`。

## アンインストール

ゲームフォルダーから `version.dll` と `Ferrite` フォルダーを削除します。一度だけ MOD なしで起動するには、起動オプションに `--no-mods` を付けます。

## MOD の作り方

- Mono ゲーム：`Ferrite.dll` と `0Harmony.dll` を参照する `net472` の C# ライブラリ。例は `examples/HelloMod`。
- IL2CPP ゲーム：`ferrite-sdk` を使う Rust の `cdylib` クレート。例は `examples/hello-native`。

詳細とコードは[英語の README](../../README.md)、バージョンごとの変更は [CHANGELOG](../../CHANGELOG.md) を参照してください。

検索キーワード：Unity MOD ローダー、Unity MOD 導入、Unity ゲーム MOD、BepInEx 代替、MelonLoader 代替、IL2CPP MOD、Mono MOD、Unity mod loader、Harmony。
