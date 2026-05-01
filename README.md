# 3LMTranslater

<p align="center">
  <img src="assets/app-icon.png" width="128" height="128" alt="App Icon">
</p>

[日本語](#日本語) | [English](#english)

---

## 日本語

3LMTranslaterは、Rust (egui) と Python を組み合わせた、プライバシー重視のローカルAI翻訳アプリケーションです。
llama-cpp-python を介して GGUF モデルをロードし、お使いの PC 上で直接翻訳を実行します。

### 主な機能
*   **オフライン翻訳**: 外部サーバーへデータを送ることなく、GGUF モデルを用いたセキュアな翻訳が可能。
*   **多言語 TTS (音声合成)**: Kokoro-82M および espeak-ng を活用し、日本語、英語、中国語、スペイン語等の多言語再生に対応。
*   **柔軟なカスタマイズ**: システムプロンプトや推論パラメータに加え、`fonts.yaml` による表示フォントの切り替えも可能。
*   **クロスプラットフォーム**: Windows および macOS で動作（ソースからのビルドが必要）。

### 動作確認済みモデル
*   **Gemma4 E4B**
*   **TranslateGemma**
*   **LFM2.5 1.2B**
*   **Bonsai 8B**

### セットアップ

#### 0. 前提条件
ビルドには以下の環境が必要です。
*   **Rust**: 1.75.0 以上 (Edition 2024対応)
*   **Python**: 3.10 以上 (llama-cpp-python および音声合成に必須)

#### 1. 仮想環境の作成と有効化 (推奨)
依存関係の衝突を避けるため、仮想環境の使用を推奨します。
```bash
python -m venv .venv

# Windows
.venv\Scripts\activate
# macOS / Linux
source .venv/bin/activate
```

#### 2. TTSモデルの準備 (Kokoro-82M)
本アプリの音声合成機能を利用するには、[Hugging Face (hexgrad/Kokoro-82M)](https://huggingface.co/hexgrad/Kokoro-82M) から以下のファイルをダウンロードし、指定の場所に配置してください。

*   **`ttsModels/` 直下に配置**:
    *   `model.onnx`
    *   `config.json`
    *   `tokenizer.json`
    *   `tokenizer_config.json`
*   **`ttsModels/voices/` 内に配置**:
    *   使用したい言語・音声の `.bin` ファイル (例: `jf_nezumi.bin`, `af_heart.bin` 等)

#### 3. espeak-ng のインストール
多言語の音素変換に必須です。
*   **Windows**: [espeak-ng GitHub](https://github.com/espeak-ng/espeak-ng/releases) からインストーラーをダウンロード。
*   **macOS**:
```bash
brew install espeak
```

#### 4. ビルド
以下のコマンドを実行すると、Pythonの依存関係（requirements.txt）のチェックとビルドが自動で行われ、`dist/` フォルダに成果物が生成されます。
```bash
python build_app.py
```
（手動でビルドする場合は `pip install -r requirements.txt` 後、 `cargo build --release` を実行してください）

### 注意事項
本アプリの音声合成機能は現状「おまけ」としての位置付けであり、Kokoro-82M モデルにのみ対応しています。 その他の TTS エンジンには対応しておりませんのでご了承ください。

### ライセンス
本プロジェクトは **GPL-3.0** ライセンスです。
詳細は `LICENSE` および `NOTICE.md` を参照してください。

---

## English

3LMTranslater is a privacy-focused local AI translation application built with Rust (egui) and Python.
It loads GGUF models via llama-cpp-python to perform translations directly on your machine.

### Features
*   **Offline Translation**: Securely translate text using GGUF models without sending data to external servers.
*   **Multi-language TTS**: Supports audio playback in languages such as Japanese, English, Chinese, and Spanish using Kokoro-82M and espeak-ng.
*   **Customizable**: Adjust system prompts, inference parameters, and UI fonts via `fonts.yaml`.
*   **Cross-Platform**: Run on Windows and macOS (Build from source).

### Confirmed Models
*   **Gemma4 E4B**
*   **TranslateGemma**
*   **LFM2.5 1.2B**
*   **Bonsai 8B**

### Setup

#### 0. Prerequisites
The following environments are required for building:
*   **Rust**: 1.75.0 or later
*   **Python**: 3.10 or later

#### 1. Create and Activate Virtual Environment (Recommended)
```bash
python -m venv .venv

# Windows
.venv\Scripts\activate
# macOS / Linux
source .venv/bin/activate
```

#### 2. Prepare TTS Models (Kokoro-82M)
To use the TTS features, download the following files from [Hugging Face (hexgrad/Kokoro-82M)](https://huggingface.co/hexgrad/Kokoro-82M) and place them in the specified directories:

*   **In `ttsModels/` root**:
    *   `model.onnx`
    *   `config.json`
    *   `tokenizer.json`
    *   `tokenizer_config.json`
*   **In `ttsModels/voices/`**:
    *   Place `.bin` files for the voices you wish to use (e.g., `af_heart.bin`, `bm_lewis.bin`).

#### 3. Install espeak-ng
Required for phonemization in multiple languages.
*   **Windows**: Download from [espeak-ng GitHub](https://github.com/espeak-ng/espeak-ng/releases).
*   **macOS**:
```bash
brew install espeak
```

#### 4. Build
Run the following command to automatically check Python dependencies and generate the distribution package in the `dist/` folder.
```bash
python build_app.py
```
(For manual builds, run `pip install -r requirements.txt` followed by `cargo build --release`)

### Note on TTS
The Text-to-Speech (TTS) feature is currently considered a secondary "bonus" feature and strictly supports the Kokoro-82M model only. Please note that other TTS engines are not supported at this time.

### License
This project is licensed under **GPL-3.0**.
See `LICENSE` and `NOTICE.md` for more details.
