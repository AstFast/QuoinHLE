<div align="center">
<img src="assets/app-icon.svg" width="148" alt="QuoinHLE icon" />

<h1 align="center">QuoinHLE</h1>

<p align="center">
  <b>English</b> |
  <a href="docs/README.zh_CN.md">简体中文</a> |
  <a href="docs/README.ja.md">日本語</a>
</p>

<p align="center">
A BREW emulator (HLE)<br/>
</p>

<p align="center">
<a href="#quick-start"><img src="https://img.shields.io/badge/Quick%20Start-Run-8b5cf6?style=for-the-badge&logo=rocket&logoColor=white" alt="Quick Start" /></a>
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
> **QuoinHLE has not had a formal release yet.**
>
> It is currently an experimental project.

------

## Quick Start

#### Command line (recommended)

###### Quick game

```bat
quoin "*.mod" --start auto --window
```

###### Specify the screen size

```bat
quoin "*.mod" --start WxH --window
```

#### GUI

Launch `quoin.exe` directly.

[^Note]: The current GUI is not a formal release; it is provided for convenient testing.

## Quick Build

```powershell
# PowerShell 7
cd .\QuoinHLE
.\build.ps1
```

## Credits

[Credits](./docs/Credit.md)

## License

Licensed under [GPL-2.0-or-later](./LICENSE).

Some parts may be covered by other licenses; check the comments in the source code.

## Project statement

[Disclaimer](./docs/DISCLAIMER.txt)

> [!Note]
> **This project is produced with artificial intelligence.**
