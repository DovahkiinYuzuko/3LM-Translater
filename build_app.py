import os
import shutil
import subprocess
import platform
import sys # Pythonの実行パスを取得するために追加

# プロジェクトの設定[cite: 11]
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

def check_python_dependencies():
    """環境に必要なライブラリが入っているか確認し、なければインストールする"""
    if os.path.exists("requirements.txt"):
        print("Checking Python dependencies via requirements.txt...")
        try:
            # sys.executable を使うことで、今動かしているPython環境に対して実行する
            subprocess.run([sys.executable, "-m", "pip", "install", "-r", "requirements.txt"], check=True)
            print("Python dependencies are up to date.")
        except subprocess.CalledProcessError:
            print("Warning: Failed to install Python dependencies. Some features might not work.")
    else:
        print("Notice: requirements.txt not found. Skipping dependency check.")

def build():
    # 0. Pythonの依存関係をチェック
    check_python_dependencies()

    # 1. 以前のビルド成果物を削除[cite: 11]
    if os.path.exists(DIST_DIR):
        print(f"Cleaning up old {DIST_DIR}...")
        shutil.rmtree(DIST_DIR)
    os.makedirs(DIST_DIR)

    # 2. Cargoビルド実行[cite: 11]
    print("Building Rust binary...")
    try:
        subprocess.run(["cargo", "build", "--release"], check=True)
    except subprocess.CalledProcessError:
        print("Build failed! Check your Rust code.")
        return

    # 3. 実行ファイルの特定[cite: 11]
    binary_ext = ".exe" if platform.system() == "Windows" else ""
    binary_name = f"{APP_NAME}{binary_ext}"
    source_binary = os.path.join("target", "release", binary_name)

    if os.path.exists(source_binary):
        shutil.copy2(source_binary, os.path.join(DIST_DIR, binary_name))
        print(f"Copied binary: {binary_name}")
    else:
        print(f"Error: Binary not found at {source_binary}")
        return

    # 4. 個別ファイルのコピー[cite: 11]
    for file in FILES_TO_COPY:
        if os.path.exists(file):
            shutil.copy2(file, os.path.join(DIST_DIR, file))
            print(f"Copied file: {file}")
        else:
            print(f"Warning: {file} not found, skipping.")

    # 5. ディレクトリのコピー[cite: 11]
    for directory in DIRS_TO_COPY:
        if os.path.exists(directory):
            shutil.copytree(directory, os.path.join(DIST_DIR, directory))
            print(f"Copied directory: {directory}/")
        else:
            print(f"Warning: {directory}/ not found, skipping.")

    print("\nBuild Complete! Check the 'dist' folder.")

if __name__ == "__main__":
    build()