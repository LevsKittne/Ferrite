# Ferrite — trình tải mod cho game Unity

[English](../../README.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [中文](README.zh-CN.md) · [Español](README.es.md) · [Português](README.pt-BR.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Türkçe](README.tr.md) · [Polski](README.pl.md) · [Italiano](README.it.md) · Tiếng Việt · [Bahasa Indonesia](README.id.md)

Ferrite là trình tải mod (mod loader) miễn phí cho mọi game làm bằng Unity, cả Mono và IL2CPP. Lõi được viết bằng Rust và cài đặt chỉ bằng một tệp `version.dll`. Đây là lựa chọn thay thế BepInEx và MelonLoader: khởi động nhanh hơn, gần như không tốn thêm bộ nhớ, cài đặt bằng một cú nhấp.

## Tính năng

- Mọi game Unity, không cần cấu hình riêng cho từng game. Mono và IL2CPP, Unity 2020–6000.
- Mod cho Mono là thư viện C# bình thường dùng Harmony. Mod cho IL2CPP là thư viện Rust (`ferrite-sdk`) có hook và truy cập các lớp của game.
- Menu trong game (F1): danh sách mod, cài đặt, console, tùy chọn trình tải. Hoạt động với Direct3D 11, Direct3D 12, Vulkan và OpenGL.
- Mod khởi động trước cảnh đầu tiên. Mod và cài đặt được tải lại mà không cần khởi động lại game.
- Vá assembly bằng Mono.Cecil, phụ thuộc có phiên bản, console lệnh, trình gỡ lỗi Mono (dnSpy, Rider).
- Trình cài đặt 12 ngôn ngữ.

## Cài đặt

1. Tải `Ferrite-Setup.exe` từ trang [Releases](https://github.com/LevsKittne/Ferrite/releases).
2. Đặt nó cạnh tệp `.exe` của game rồi chạy, hoặc chạy ở bất kỳ đâu và chọn thư mục game.
3. Bỏ mod vào `Ferrite\mods`.
4. Mở game. Nhấn F1 để mở menu.

Cài đặt im lặng: `Ferrite-Setup.exe /silent "/dir=C:\duong\dan\game"`.

## Gỡ cài đặt

Xóa `version.dll` và thư mục `Ferrite` khỏi thư mục game. Để chạy game một lần không có mod, thêm tham số `--no-mods`.

## Viết mod

- Game Mono: thư viện C# `net472` tham chiếu `Ferrite.dll` và `0Harmony.dll`, ví dụ trong `examples/HelloMod`.
- Game IL2CPP: crate Rust `cdylib` dùng `ferrite-sdk`, ví dụ trong `examples/hello-native`.

Chi tiết và mã nguồn trong [README tiếng Anh](../../README.md). Thay đổi theo phiên bản trong [CHANGELOG](../../CHANGELOG.md).

Từ khóa tìm kiếm: trình tải mod Unity, cài mod game Unity, mod loader Unity, thay thế BepInEx, thay thế MelonLoader, mod IL2CPP, mod Mono, Harmony.
