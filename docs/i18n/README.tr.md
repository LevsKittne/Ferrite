# Ferrite — Unity oyunları için mod yükleyici

[English](../../README.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [中文](README.zh-CN.md) · [Español](README.es.md) · [Português](README.pt-BR.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · Türkçe · [Polski](README.pl.md) · [Italiano](README.it.md) · [Tiếng Việt](README.vi.md) · [Bahasa Indonesia](README.id.md)

Ferrite, Unity ile yapılmış her oyun için (Mono ve IL2CPP) ücretsiz bir mod yükleyicidir (mod loader). Çekirdeği Rust ile yazılmıştır ve tek bir `version.dll` dosyasıyla kurulur. BepInEx ve MelonLoader'a bir alternatiftir: daha hızlı açılır, neredeyse hiç ek bellek kullanmaz, tek tıkla kurulur.

## Özellikler

- Oyuna özel ayar gerekmeden her Unity oyunu. Mono ve IL2CPP, Unity 2020–6000.
- Mono modları Harmony kullanan normal C# kütüphaneleridir. IL2CPP modları, hook ve oyun sınıflarına erişim sunan Rust kütüphaneleridir (`ferrite-sdk`).
- Oyun içi menü (F1): modlar, ayarlar, konsol, yükleyici seçenekleri. Direct3D 11, Direct3D 12, Vulkan ve OpenGL ile çalışır.
- Modlar ilk sahneden önce başlar. Modlar ve ayarları oyunu yeniden başlatmadan yeniden yüklenir.
- Mono.Cecil ile assembly yamaları, sürümlü bağımlılıklar, komut konsolu, Mono hata ayıklayıcı (dnSpy, Rider).
- 12 dilde kurulum programı (Türkçe dahil).

## Kurulum

1. [Releases](https://github.com/LevsKittne/Ferrite/releases) sayfasından `Ferrite-Setup.exe` dosyasını indirin.
2. Oyunun `.exe` dosyasının yanına koyup çalıştırın veya herhangi bir yerde çalıştırıp oyun klasörünü seçin.
3. Modları `Ferrite\mods` klasörüne koyun.
4. Oyunu başlatın. Menü F1 ile açılır.

Sessiz kurulum: `Ferrite-Setup.exe /silent "/dir=C:\oyun\yolu"`.

## Kaldırma

Oyun klasöründen `version.dll` dosyasını ve `Ferrite` klasörünü silin. Oyunu bir kez modsuz başlatmak için `--no-mods` parametresini ekleyin.

## Mod yazma

- Mono oyunları: `Ferrite.dll` ve `0Harmony.dll` referanslı `net472` C# kütüphanesi, örnek `examples/HelloMod`.
- IL2CPP oyunları: `ferrite-sdk` kullanan Rust `cdylib` crate'i, örnek `examples/hello-native`.

Ayrıntılar ve kod [İngilizce README](../../README.md) içinde, sürüm değişiklikleri [CHANGELOG](../../CHANGELOG.md) içinde.

Şu şekilde de aranır: Unity mod yükleyici, Unity mod loader, Unity oyun modları, BepInEx alternatifi, MelonLoader alternatifi, IL2CPP mod, Mono mod, Harmony.
