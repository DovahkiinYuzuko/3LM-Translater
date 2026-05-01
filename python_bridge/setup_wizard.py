import os
import sys
import platform
import subprocess
import shutil

def is_nvidia_gpu_available():
    """NVIDIAのGPU（CUDA）が利用可能かチェック"""
    return shutil.which("nvidia-smi") is not None

def is_macos_arm():
    """Apple Silicon (M1/M2/M3) かチェック"""
    return platform.system() == "Darwin" and platform.machine() == "arm64"

def get_best_cmake_args():
    """OSとハードウェアに合わせて最適なCMAKE_ARGSを生成"""
    system = platform.system()
    
    if is_nvidia_gpu_available():
        print("Detected NVIDIA GPU. Preparing CUDA build...")
        return "-DGGML_CUDA=on"
    
    if is_macos_arm():
        print("Detected Apple Silicon. Preparing Metal build...")
        return "-DGGML_METAL=on"
    
    print("No specialized GPU detected. Falling back to CPU build.")
    return "-DGGML_CPU=on"

def run_setup():
    """実際のインストール作業を実行"""
    print("--- 3LMTranslater Auto Setup Wizard ---")
    
    # 1. pipのアップグレード
    print("Upgrading pip...")
    subprocess.run([sys.executable, "-m", "pip", "install", "--upgrade", "pip"], check=True)

    # 2. 必要なビルドツール（ninja等）の確認
    print("Installing build dependencies (ninja)...")
    subprocess.run([sys.executable, "-m", "pip", "install", "ninja"], check=True)

    # 3. 最適な引数の取得
    cmake_args = get_best_cmake_args()
    
    # 4. llama-cpp-pythonのインストール実行
    # Windowsの場合はNinjaを強制指定すると安定する
    if platform.system() == "Windows" and "-DGGML_CUDA=on" in cmake_args:
        cmake_args += " -GNinja"

    env = os.environ.copy()
    env["CMAKE_ARGS"] = cmake_args
    
    print(f"Executing build with CMAKE_ARGS: {cmake_args}")
    print("This may take several minutes. Please wait...")
    
    try:
        # キャッシュを無視して最新ソースからビルド
        subprocess.run([
            sys.executable, "-m", "pip", "install", 
            "llama-cpp-python", 
            "--upgrade", "--force-reinstall", "--no-cache-dir"
        ], env=env, check=True)
        print("\nSetup successful! Your engine is now optimized.")
        return True
    except subprocess.CalledProcessError as e:
        print(f"\nSetup failed with error: {e}")
        if platform.system() == "Windows":
            print("\n[Tip] On Windows, ensure you are running this from a 'Developer PowerShell' or have MSVC installed.")
        return False

if __name__ == "__main__":
    run_setup()