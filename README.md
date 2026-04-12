<img width="200px" src="public/icon.svg" align="left"/>

# Saladict

> A cross-platform text selection translator

![License](https://img.shields.io/github/license/allentown521/saladict.svg)
![Tauri](https://img.shields.io/badge/Tauri-2.x-blue?logo=tauri)
![JavaScript](https://img.shields.io/badge/-JavaScript-yellow?logo=javascript&logoColor=white)
![Rust](https://img.shields.io/badge/-Rust-orange?logo=rust&logoColor=white)
![Windows](https://img.shields.io/badge/-Windows-blue?logo=windows&logoColor=white)
![MacOS](https://img.shields.io/badge/-macOS-black?&logo=apple&logoColor=white)
![Linux](https://img.shields.io/badge/-Linux-yellow?logo=linux&logoColor=white)

<br/>
<hr/>
<div align="center">

<h3>English | <a href='./README_CN.md'>中文</a> | <a href='./README_KR.md'> 한글 </a></h3>

<table>
<tr>
    <td> <img src="asset/1.png">
    <td> <img src="asset/2.png">
    <td> <img src="asset/3.png">
</table>

# Table of Contents

</div>

-   [Usage](#usage)
-   [Features](#features)
-   [Supported Services](#supported-services)
-   [Plugin System](#plugin-system)
-   [Installation](#installation)
-   [External Calls](#external-calls)
-   [Wayland Support](#wayland-support)
-   [Internationalization](#internationalizationweblate)
-   [Contributors](#contributors)

<div align="center">

# Why is Saladict a better alternative to Pot Translate?

</div>

Saladict is a fork of Pot Translation. Since Pot Translation is already excellent, why choose Saladict instead? Here are the main reasons:

-   **More stable**: Many pull requests in Pot remain unmerged, including important bug fixes. Saladict regularly syncs fixes from the upstream Pot repository and selectively integrates new features to ensure overall stability.

-   **Broader AI translation integration**: Starting from v4.0.0, Saladict offers built-in online translation services. Users no longer need to apply for their own API keys.

-   **Unified product ecosystem**: Our long-term vision is to maintain consistency between the browser extension and desktop application, delivering the most complete text-selection translation solution. Many users first encounter Saladict through the Chrome extension before downloading the desktop app.

-   **Longer support cycle**: Pot's maintenance frequency has been declining. Saladict generates revenue through ads and online translation services, giving us the motivation to continue maintaining the product while keeping the code open.

# Usage

| Selection Translation | Input Translation | External Calls |
| --- | --- | --- |
| Select text and press the shortcut or click the quick translation icon | Press the input translation shortcut to open the translation window, enter text and press Enter | Integrate with other software for a more efficient workflow, see [External Calls](#external-calls) |
| <img src="asset/eg1.gif"/> | <img src="asset/eg2.gif"/> | <img src="asset/eg3.gif"/> |

| Clipboard Listening | Screenshot OCR | Screenshot Translation |
| --- | --- | --- |
| Right-click the tray icon and select `Clipboard Listening` to start. Copied text will be translated automatically. | Press the Screenshot OCR shortcut and select the area to recognize | Press the Screenshot Translation shortcut and select the area to translate |
| <img src="asset/eg4.gif"/> | <img src="asset/eg5.gif"/> | <img src="asset/eg6.gif"/> |

<div align="center">

# Features

</div>

-   [x] Parallel translations with multiple services ([Supported Services](#supported-services))
-   [x] Multi-service OCR ([Supported Services](#supported-services))
-   [x] Multi-service Text-to-Speech ([Supported Services](#supported-services))
-   [x] Export to vocabulary apps ([Supported Services](#supported-services))
-   [x] External calls ([details](#external-calls))
-   [x] Plugin system ([Plugin System](#plugin-system))
-   [x] All PC platforms supported (Windows, macOS, Linux)
-   [x] Wayland support (tested on KDE, Gnome, and Hyprland)
-   [x] Multi-language support

<div align="center">

# Supported Services

</div>

## Translation

-   [x] [OpenAI](https://platform.openai.com/)
-   [x] [ChatGLM (Zhipu AI)](https://www.zhipuai.cn/)
-   [x] [Gemini](https://gemini.google.com/)
-   [x] [Ollama](https://www.ollama.com/) (Offline)
-   [x] [Ali Translate](https://www.aliyun.com/product/ai/alimt)
-   [x] [Baidu Translate](https://fanyi.baidu.com/)
-   [x] [Caiyun](https://fanyi.caiyunapp.com/)
-   [x] [Tencent Translator](https://fanyi.qq.com/)
-   [x] [Tencent Interactive Translate](https://transmart.qq.com/)
-   [x] [Volcengine Translate](https://translate.volcengine.com/)
-   [x] [NiuTrans](https://niutrans.com/)
-   [x] [Google Translate](https://translate.google.com)
-   [x] [Bing Translate](https://learn.microsoft.com/en-us/azure/cognitive-services/translator/)
-   [x] [Bing Dictionary](https://www.bing.com/dict)
-   [x] [DeepL](https://www.deepl.com/)
-   [x] [Youdao](https://ai.youdao.com/)
-   [x] [Cambridge Dictionary](https://dictionary.cambridge.org/)
-   [x] [Yandex](https://translate.yandex.com/)
-   [x] [Lingva](https://github.com/TheDavidDelta/lingva-translate) ([Plugin](https://github.com/pot-app/pot-app-translate-plugin-template))
-   [x] [Tatoeba](https://tatoeba.org/) ([Plugin](https://github.com/pot-app/pot-app-translate-plugin-tatoeba))
-   [x] [ECDICT](https://github.com/skywind3000/ECDICT) ([Plugin](https://github.com/pot-app/pot-app-translate-plugin-ecdict))

More services available via the [Plugin System](#plugin-system)

## Text Recognition (OCR)

-   [x] System OCR (Offline)
    -   [x] [Windows.Media.OCR](https://learn.microsoft.com/en-us/uwp/api/windows.media.ocr.ocrengine?view=winrt-22621) on Windows
    -   [x] [Apple Vision Framework](https://developer.apple.com/documentation/vision/recognizing_text_in_images) on macOS
    -   [x] [Tesseract OCR](https://github.com/tesseract-ocr) on Linux
-   [x] [Tesseract.js](https://tesseract.projectnaptha.com/) (Offline)
-   [x] [Baidu OCR](https://ai.baidu.com/tech/ocr/general)
-   [x] [Tencent OCR](https://cloud.tencent.com/product/ocr-catalog)
-   [x] [Volcengine OCR](https://www.volcengine.com/product/OCR)
-   [x] [iFlytek OCR](https://www.xfyun.cn/services/common-ocr)
-   [x] [Tencent Image Translate](https://cloud.tencent.com/document/product/551/17232)
-   [x] [Baidu Image Translate](https://fanyi-api.baidu.com/product/22)
-   [x] [Simple LaTeX](https://simpletex.cn/)
-   [x] [OCRSpace](https://ocr.space/) ([Plugin](https://github.com/pot-app/pot-app-recognize-plugin-template))
-   [x] [Rapid](https://github.com/RapidAI/RapidOcrOnnx) (Offline [Plugin](https://github.com/pot-app/pot-app-recognize-plugin-rapid))
-   [x] [Paddle](https://github.com/hiroi-sora/PaddleOCR-json) (Offline [Plugin](https://github.com/pot-app/pot-app-recognize-plugin-paddle))

More services available via the [Plugin System](#plugin-system)

## Text-to-Speech

-   [x] [Lingva](https://github.com/thedaviddelta/lingva-translate)

More services available via the [Plugin System](#plugin-system)

## Vocabulary / Collection

-   [x] [Anki](https://apps.ankiweb.net/)
-   [x] [Eudic](https://dict.eudic.net/)
-   [x] [Youdao](https://www.youdao.com/) ([Plugin](https://github.com/pot-app/pot-app-collection-plugin-youdao))
-   [x] [ShanBay](https://web.shanbay.com/web/main) ([Plugin](https://github.com/pot-app/pot-app-collection-plugin-shanbay))

More services available via the [Plugin System](#plugin-system)

<div align="center">

# Plugin System

</div>

The built-in services are limited, but you can extend the app's functionality through the plugin system.

## Installing Plugins

You can find plugins in the [Plugin List](https://app.saladict.net/plugin.html), then download them from the plugin repository.

Saladict plugins use the `.potext` extension. After downloading a `.potext` file, go to **Preferences > Service Settings > Add External Plugin > Install External Plugin**, select the `.potext` file to install it. Once added to the service list, it works just like a built-in service.

### Troubleshooting

-   **The specified module could not be found** (Windows)

    This error occurs because the system lacks C++ runtime libraries. Download and install them from [here](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist?view=msvc-170#visual-studio-2015-2017-2019-and-2022).

-   **Not a valid Win32 application** (Windows)

    This error means you downloaded a plugin for the wrong architecture. Go to the plugin repository and download the correct version.

## Developing Plugins

The [Template](https://app.saladict.net/plugin.html#%E6%A8%A1%E6%9D%BF) section in the [Plugin List](https://app.saladict.net/plugin.html) provides development templates for various plugin types. See the corresponding template repository for documentation.

<div align="center">

# Installation

</div>

## Windows

### Microsoft Store

 <a href="https://apps.microsoft.com/detail/9pfzvl2bqx1s" target="_blank">
  <img src="asset/download_on_microsoft_store.png" alt="Download on the Microsoft Store" style="width: 156px;" />
 </a>

### Via Winget

```powershell
winget install allentown521.Saladict
```

### Manual Install

1. Download the latest `.exe` installer from the [Release](https://github.com/allentown521/saladict/releases/latest) page.

    - 64-bit: `saladict_{version}_x64-setup.exe`
    - 32-bit: `saladict_{version}_x86-setup.exe`
    - ARM64: `saladict_{version}_arm64-setup.exe`

2. Double-click the installer to install.

### Troubleshooting

-   **No UI after launch, tray icon unresponsive**

    Check if WebView2 has been uninstalled or disabled. If so, reinstall or re-enable it.

    For enterprise systems where WebView2 cannot be installed, download the bundled WebView2 version from [Release](https://github.com/allentown521/saladict/releases/latest): `saladict_{version}_{arch}_fix_webview2_runtime-setup.exe`

    If the issue persists, try launching in Windows 7 compatibility mode.

## macOS

### Mac App Store

For M-series Macs, install from the Mac App Store:

 <a href="https://apps.apple.com/app/6740262076" target="_blank">
  <img src="asset/download_on_mac_app_store.svg" alt="Download on the Mac App Store" style="width: 156px;" />
 </a>

> Due to technical limitations, the Mac App Store version does not support `Selection Translation` or `Shortcut Selection Translation`.

### Manual Install

1. Download the latest `.dmg` from the [Release](https://github.com/allentown521/saladict/releases/latest) page. (M1/M2: download `saladict_{version}_aarch64.dmg`; Intel: download `saladict_{version}_x64.dmg`)
2. Open the `.dmg` and drag Saladict into the Applications folder.

### Via Homebrew

1. Add the tap:

```bash
brew tap allentown521/homebrew-saladict
```

2. Install:

```bash
brew install --cask saladict
```

3. Upgrade:

```bash
brew upgrade --cask saladict
```

### Troubleshooting

-   If you get an accessibility permission prompt every time, or selection translation doesn't work, go to **Settings > Privacy & Security > Accessibility**, remove "Saladict", then re-add it.

## Linux

### Debian/Ubuntu

1. Download the latest `.deb` package for your architecture from the [Release](https://github.com/allentown521/saladict/releases/latest) page.

### NixOS

A `shell.nix` is included in the repository for building on NixOS:

```bash
nix-shell
pnpm tauri build
```

<div align="center">

# External Calls

</div>

Saladict provides a complete HTTP interface for integration with other software. Send HTTP requests to `127.0.0.1:port`, where `port` is the listening port (default: `60606`, configurable in settings).

## API

```bash
POST "/" => Translate given text (body is the text to translate)
GET "/config" => Open settings
POST "/translate" => Translate given text (same as "/")
GET "/selection_translate" => Selection translation
GET "/input_translate" => Input translation
GET "/ocr_recognize" => Screenshot OCR
GET "/ocr_translate" => Screenshot translation
GET "/ocr_recognize?screenshot=false" => OCR without built-in screenshot
GET "/ocr_translate?screenshot=false" => Translation without built-in screenshot
GET "/ocr_recognize?screenshot=true" => Screenshot OCR
GET "/ocr_translate?screenshot=true" => Screenshot translation
```

## Example

-   Trigger selection translation:

    ```bash
    curl "127.0.0.1:60606/selection_translate"
    ```

## Using an External Screenshot Tool

This feature lets you use your own screenshot tool instead of the built-in one, which also solves issues where Saladict's built-in screenshot doesn't work on some platforms.

### Workflow

1. Take a screenshot using your preferred tool
2. Save it to `$CACHE/allen.town.focus.saladict/pot_screenshot_cut.png`
3. Send a request to `127.0.0.1:port/ocr_recognize?screenshot=false`

> `$CACHE` is the system cache directory, e.g. `C:\Users\{username}\AppData\Local\allen.town.focus.saladict\pot_screenshot_cut.png` on Windows.

### Example (Linux with Flameshot)

```bash
rm ~/.cache/allen.town.focus.saladict/pot_screenshot_cut.png && flameshot gui -s -p ~/.cache/allen.town.focus.saladict/pot_screenshot_cut.png && curl "127.0.0.1:60606/ocr_recognize?screenshot=false"
```

## Quick Selection Translation Integrations

### SnipDo (Windows)

1. Install [SnipDo](https://apps.microsoft.com/store/detail/snipdo/9NPZ2TVKJVT7) from the Microsoft Store.
2. Download the Saladict SnipDo extension (`Saladict.pbar`) from the [Release](https://github.com/allentown521/saladict/releases/latest) page.
3. Double-click the extension file to install.
4. Select text to see the SnipDo toolbar; click the translate button.

### PopClip (macOS)

1. Install [PopClip](https://www.popclip.app/) from the official website.
2. Download the Saladict PopClip extension (`Saladict.popclipextz`) from the [Release](https://github.com/allentown521/saladict/releases/latest) page.
3. Double-click the extension file to install.
4. Enable the Saladict extension in PopClip settings, then select text to translate.

### Starry (Linux)

> Starry is still in development; you'll need to compile it manually.

GitHub: [ccslykx/Starry](https://github.com/ccslykx/Starry)

<div align="center">

# Wayland Support

</div>

Due to varying Wayland support across distributions, Saladict can't achieve perfect compatibility out of the box. Below are solutions to common issues.

```
Ubuntu 22.04+ defaults to Wayland. To use X11 instead:
1. Click the gear icon on the login screen
2. Select "Ubuntu on Xorg"
3. Log in

This resolves the following issues:
```

## Shortcuts Don't Work

Tauri's shortcut system doesn't support Wayland natively. Set up system-level shortcuts that send `curl` requests to Saladict instead. See [External Calls](#external-calls).

## Screenshot Doesn't Work

On pure Wayland compositors (e.g. Hyprland), the built-in screenshot doesn't work. Use an external screenshot tool instead. See [Using an External Screenshot Tool](#using-an-external-screenshot-tool).

Example for Hyprland (using `grim` and `slurp`):

```conf
bind = ALT, X, exec, grim -g "$(slurp)" ~/.cache/allen.town.focus.saladict/pot_screenshot_cut.png && curl "127.0.0.1:60606/ocr_recognize?screenshot=false"
bind = ALT, C, exec, grim -g "$(slurp)" ~/.cache/allen.town.focus.saladict/pot_screenshot_cut.png && curl "127.0.0.1:60606/ocr_translate?screenshot=false"
```

## Translation Window Follows Mouse

Saladict currently can't get accurate mouse coordinates under Wayland. For some compositors, you can set window rules. Hyprland example:

```conf
windowrulev2 = float, class:(saladict), title:(Translator|OCR|PopClip|Screenshot Translate)
windowrulev2 = move cursor 0 0, class:(saladict), title:(Translator|PopClip|Screenshot Translate)
```

<div align="center">

# Internationalization ([Weblate](https://hosted.weblate.org/engage/saladict-app/))

[![](https://hosted.weblate.org/widget/allentown521/saladict/svg-badge.svg)](https://hosted.weblate.org/engage/saladict-app/)

[![](https://hosted.weblate.org/widget/allentown521/saladict/zh_Hans/multi-auto.svg)](https://hosted.weblate.org/engage/saladict-app/)

</div>

<div align="center">

# Contributors

</div>

<img src="https://github.com/pot-app/.github/blob/master/pot-desktop-contributions.svg?raw=true" width="100%"/>

## Building from Source

### Requirements

Node.js >= 18.0.0

pnpm >= 8.5.0

Rust >= 1.80.0

### Build Steps

1. Clone the repository

    ```bash
    git clone https://github.com/allentown521/saladict.git
    ```

2. Install dependencies

    ```bash
    cd saladict
    pnpm install
    ```

3. Install system dependencies (Linux only)

    ```bash
    # Debian/Ubuntu
    sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf libxdo-dev libxcb1 libxrandr2 libdbus-1-3 libssl-dev libsoup-3.0-dev
    ```

    NixOS users can use the included `shell.nix`:

    ```bash
    nix-shell
    ```

4. Development

    ```bash
    pnpm tauri dev
    ```

5. Build

    ```bash
    pnpm tauri build
    ```

<div align="center">

# Acknowledgements

</div>

-   [Bob](https://github.com/ripperhe/Bob) - Inspiration
-   [bob-plugin-openai-translator](https://github.com/yetone/bob-plugin-openai-translator) - OpenAI API reference
-   [@uiYzzi](https://github.com/uiYzzi) - Implementation ideas
-   [@Lichenkass](https://github.com/Lichenkass) - Maintaining the Deepin App Store
-   [Tauri](https://github.com/tauri-apps/tauri) - GUI framework

<div align="center">
</div>
