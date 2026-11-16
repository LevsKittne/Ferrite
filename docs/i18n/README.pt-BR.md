# Ferrite — carregador de mods para jogos Unity

[English](../../README.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [中文](README.zh-CN.md) · [Español](README.es.md) · Português · [Deutsch](README.de.md) · [Français](README.fr.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Türkçe](README.tr.md) · [Polski](README.pl.md) · [Italiano](README.it.md) · [Tiếng Việt](README.vi.md) · [Bahasa Indonesia](README.id.md)

Ferrite é um carregador de mods (mod loader) gratuito para qualquer jogo feito em Unity, Mono e IL2CPP. O núcleo é escrito em Rust e é instalado com um único arquivo `version.dll`. É uma alternativa ao BepInEx e ao MelonLoader: inicia mais rápido, quase não usa memória e instala com um clique.

## Recursos

- Qualquer jogo Unity, sem configuração por jogo. Mono e IL2CPP, Unity 2020–6000.
- Mods para Mono são bibliotecas C# comuns com Harmony. Mods para IL2CPP são bibliotecas em Rust (`ferrite-sdk`) com hooks e acesso às classes do jogo.
- Menu dentro do jogo (F1): mods, configurações, console e opções do carregador. Funciona com Direct3D 11, Direct3D 12, Vulkan e OpenGL.
- Os mods iniciam antes da primeira cena. Mods e configurações recarregam sem reiniciar o jogo.
- Patchers de assemblies com Mono.Cecil, dependências com versão, console de comandos, depurador Mono (dnSpy, Rider).
- Instalador em 12 idiomas.

## Instalação

1. Baixe `Ferrite-Setup.exe` em [Releases](https://github.com/LevsKittne/Ferrite/releases).
2. Coloque-o ao lado do `.exe` do jogo e execute, ou execute em qualquer lugar e escolha a pasta do jogo.
3. Coloque os mods em `Ferrite\mods`.
4. Inicie o jogo. O menu abre com F1.

Instalação silenciosa: `Ferrite-Setup.exe /silent "/dir=C:\caminho\do\jogo"`.

## Desinstalação

Apague `version.dll` e a pasta `Ferrite` da pasta do jogo. Para iniciar o jogo uma vez sem mods, adicione o parâmetro `--no-mods`.

## Criar mods

- Jogos Mono: biblioteca C# `net472` que referencia `Ferrite.dll` e `0Harmony.dll`, exemplo em `examples/HelloMod`.
- Jogos IL2CPP: crate Rust `cdylib` com `ferrite-sdk`, exemplo em `examples/hello-native`.

Detalhes e código no [README em inglês](../../README.md). Mudanças por versão no [CHANGELOG](../../CHANGELOG.md).

Também procurado como: carregador de mods Unity, mod loader Unity, mods para jogos Unity, alternativa ao BepInEx, alternativa ao MelonLoader, mods IL2CPP, mods Mono, Harmony.
