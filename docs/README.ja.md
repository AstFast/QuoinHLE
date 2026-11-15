<div align="center">
<img src="../assets/app-icon.svg" width="148" alt="QuoinHLE icon" />

<h1 align="center">QuoinHLE</h1>

<p align="center">
  <a href="../README.md">English</a> |
  <a href="README.zh_CN.md">简体中文</a> |
  <b>日本語</b>
</p>

<p align="center">
BREW エミュレーター（HLE）<br/>
</p>

<p align="center">
<a href="#クイックスタート"><img src="https://img.shields.io/badge/Quick%20Start-Run-8b5cf6?style=for-the-badge&logo=rocket&logoColor=white" alt="Quick Start" /></a>
<a><img src="https://img.shields.io/badge/Architecture-HLE-0ea5e9?style=for-the-badge&logo=diagram&logoColor=white" alt="Architecture" /></a>
</p>

<p align="center">
<img src="https://img.shields.io/badge/version-0.0.1--alpha-ff69b4?style=flat-square" alt="Version" />
<img src="https://img.shields.io/badge/Rust-2021-f97316?style=flat-square&logo=rust&logoColor=white" alt="Rust" />
<img src="https://img.shields.io/badge/Windows-10%2B-0078D4?style=flat-square&logo=windows&logoColor=white" alt="Windows 10+" />
<img src="https://img.shields.io/badge/license-GPL--2.0--or--later-blue?style=flat-square&logo=gnuproject&logoColor=white" alt="License GPL-2.0-or-later" />
</p>


---

> [!IMPORTANT]
> **QuoinHLE はまだ正式リリースされていません。**
>
> 現在は実験的なプロジェクトです。

------

## クイックスタート

#### コマンドライン（推奨）

###### すぐに起動

```bat
quoin "*.mod" --start auto --window
```

###### サイズを指定

```bat
quoin "*.mod" --start WxH --window
```

#### GUI

`quoin.exe` を直接起動してください。

[^Note]: 現在の GUI は正式版ではなく、テスト用です。

## クイックビルド

```powershell
# PowerShell 7
cd .\QuoinHLE
.\build.ps1
```

## 謝辞

[クレジット](./Credit.md)

## ライセンス

[GPL-2.0-or-later](../LICENSE) の下でライセンスされています。

一部の部分には別のライセンスが適用される場合があります。ソースコードのコメントを確認してください。

## プロジェクト声明

[免責事項](./DISCLAIMER.txt)

> [!Note]
> **本プロジェクトは AI を使用して制作されています。**
