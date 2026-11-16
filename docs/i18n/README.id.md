# Ferrite — pemuat mod untuk game Unity

[English](../../README.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [中文](README.zh-CN.md) · [Español](README.es.md) · [Português](README.pt-BR.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Türkçe](README.tr.md) · [Polski](README.pl.md) · [Italiano](README.it.md) · [Tiếng Việt](README.vi.md) · Bahasa Indonesia

Ferrite adalah pemuat mod (mod loader) gratis untuk semua game buatan Unity, Mono maupun IL2CPP. Intinya ditulis dengan Rust dan dipasang hanya dengan satu file `version.dll`. Ini alternatif untuk BepInEx dan MelonLoader: mulai lebih cepat, hampir tidak memakai memori tambahan, dipasang dengan satu klik.

## Fitur

- Semua game Unity, tanpa pengaturan khusus per game. Mono dan IL2CPP, Unity 2020–6000.
- Mod untuk Mono adalah pustaka C# biasa dengan Harmony. Mod untuk IL2CPP adalah pustaka Rust (`ferrite-sdk`) dengan hook dan akses ke kelas game.
- Menu di dalam game (F1): mod, pengaturan, konsol, opsi pemuat. Bekerja dengan Direct3D 11, Direct3D 12, Vulkan, dan OpenGL.
- Mod berjalan sebelum adegan pertama. Mod dan pengaturannya dimuat ulang tanpa memulai ulang game.
- Patcher assembly dengan Mono.Cecil, dependensi dengan versi, konsol perintah, debugger Mono (dnSpy, Rider).
- Penginstal dalam 12 bahasa.

## Pemasangan

1. Unduh `Ferrite-Setup.exe` dari halaman [Releases](https://github.com/LevsKittne/Ferrite/releases).
2. Taruh di samping file `.exe` game lalu jalankan, atau jalankan di mana saja dan pilih folder game.
3. Taruh mod di `Ferrite\mods`.
4. Jalankan game. Menu terbuka dengan F1.

Pemasangan senyap: `Ferrite-Setup.exe /silent "/dir=C:\path\ke\game"`.

## Menghapus

Hapus `version.dll` dan folder `Ferrite` dari folder game. Untuk menjalankan game sekali tanpa mod, tambahkan parameter `--no-mods`.

## Membuat mod

- Game Mono: pustaka C# `net472` yang mereferensikan `Ferrite.dll` dan `0Harmony.dll`, contoh di `examples/HelloMod`.
- Game IL2CPP: crate Rust `cdylib` dengan `ferrite-sdk`, contoh di `examples/hello-native`.

Detail dan kode ada di [README bahasa Inggris](../../README.md). Perubahan per versi ada di [CHANGELOG](../../CHANGELOG.md).

Juga dicari sebagai: pemuat mod Unity, mod loader Unity, cara pasang mod game Unity, alternatif BepInEx, alternatif MelonLoader, mod IL2CPP, mod Mono, Harmony.
