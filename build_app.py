import os
import shutil
import subprocess
import platform

# プロジェクトの設定
APP_NAME = "LLLMTranslater"
DIST_DIR = "dist"
FILES_TO_COPY = [
    "config.yaml",
    "fonts.yaml",
    "i18n.yaml",
]
DIRS_TO_COPY = [
    "fonts",
    "python_bridge",
    "ttsModels"
]

def build():
    # 1. 以前のビルド成果物を削除
    if os.path.exists(DIST_DIR):
        print(f"Cleaning up old {DIST_DIR}...")
        shutil.rmtree(DIST_DIR)
    os.makedirs(DIST_DIR)

    # 2. Cargoビルド実行
    print("Building Rust binary...")
    try:
        subprocess.run(["cargo", "build", "--release"], check=True)
    except subprocess.CalledProcessError:
        print("Build failed! Check your Rust code.")
        return

    # 3. 実行ファイルの特定
    binary_ext = ".exe" if platform.system() == "Windows" else ""
    binary_name = f"{APP_NAME}{binary_ext}"
    source_binary = os.path.join("target", "release", binary_name)

    if os.path.exists(source_binary):
        shutil.copy2(source_binary, os.path.join(DIST_DIR, binary_name))
        print(f"Copied binary: {binary_name}")
    else:
        print(f"Error: Binary not found at {source_binary}")
        return

    # 4. 個別ファイルのコピー
    for file in FILES_TO_COPY:
        if os.path.exists(file):
            shutil.copy2(file, os.path.join(DIST_DIR, file))
            print(f"Copied file: {file}")
        else:
            print(f"Warning: {file} not found, skipping.")

    # 5. ディレクトリのコピー
    for directory in DIRS_TO_COPY:
        if os.path.exists(directory):
            shutil.copytree(directory, os.path.join(DIST_DIR, directory))
            print(f"Copied directory: {directory}/")
        else:
            print(f"Warning: {directory}/ not found, skipping.")

    print("\nBuild Complete! Check the 'dist' folder.")

if __name__ == "__main__":
    build()