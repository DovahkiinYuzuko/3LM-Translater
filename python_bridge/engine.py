import os
import json
import typing
import re
import numpy as np
import onnxruntime # type: ignore
from llama_cpp import Llama

_model = None
_ort_session = None
_vocab = None
_g2p_ja = None
_g2p_en = None
_phonemizer = None

def load_model(absolute_model_path):
    global _model
    if _model is not None and getattr(_model, "model_path", "") == absolute_model_path:
        return
    print(f"Loading via llama.cpp: {absolute_model_path}")
    _model = Llama(model_path=absolute_model_path, n_gpu_layers=-1, n_ctx=8192, verbose=False)

def translate(text, params):
    global _model
    if _model is None: return "Error: Model not loaded"
    src, target = params["src_lang"], params["target_lang"]
    sys_prompt = params.get("system_prompt", "Translate the following input.")
    prompt = f"### Instruction:\n{sys_prompt}\n[Source: {src} -> Target: {target}]\n\n### Input:\n{text}\n\n### Response:\n"
    
    callback = params.get("callback")

    output = _model(
        prompt, 
        max_tokens=params["max_new_tokens"], 
        temperature=params["temperature"], 
        stop=["### Instruction:", "User:"],
        stream=True
    )
    
    res = ""
    for chunk in output:
        text_chunk = typing.cast(typing.Any, chunk)["choices"][0]["text"]
        res += text_chunk
        if callback:
            callback(text_chunk)
            
    return res.strip()

# espeak-ngのパスを探して環境変数にブチ込む大作戦
def setup_espeak_path(custom_path=""):
    if custom_path and os.path.exists(custom_path):
        os.environ["PATH"] = custom_path + os.pathsep + os.environ.get("PATH", "")
        print(f"espeak-ng custom path set: {custom_path}")
        return

    # 自動スキャン（Windowsのよくある場所）
    default_paths = [
        r"C:\Program Files\eSpeak NG",
        r"C:\Program Files (x86)\eSpeak NG"
    ]
    for path in default_paths:
        if os.path.exists(os.path.join(path, "espeak-ng.exe")):
            os.environ["PATH"] = path + os.pathsep + os.environ.get("PATH", "")
            print(f"espeak-ng auto-detected at: {path}")
            return
            
    print("espeak-ng not found automatically. If TTS fails, set path in config.yaml.")

def load_tts_resources():
    global _ort_session, _vocab, _g2p_ja, _g2p_en, _phonemizer
    if _ort_session is None:
        print("Loading Raw ONNX TTS components...")
        _ort_session = onnxruntime.InferenceSession("ttsModels/model.onnx")
        
        with open("ttsModels/tokenizer.json", "r", encoding="utf-8") as f:
            tokenizer_data = json.load(f)
            if "model" in tokenizer_data and "vocab" in tokenizer_data["model"]:
                _vocab = tokenizer_data["model"]["vocab"]
            else:
                print("Error: 'vocab' not found in tokenizer.json")
                _vocab = {}

        try:
            from misaki import ja, en # type: ignore
            _g2p_ja = ja.JAG2P()
            _g2p_en = en.G2P()
        except ImportError:
            print("Warning: 'misaki' not found.")

        try:
            from phonemizer.backend import EspeakBackend # type: ignore
            _phonemizer = EspeakBackend
            print("phonemizer (espeak-ng) loaded for multi-language support!")
        except ImportError:
            print("Warning: 'phonemizer' not found. Multi-language TTS may not work. Run: pip install phonemizer")
        except Exception as e:
            print(f"Warning: phonemizer init error: {e}. Is espeak-ng installed?")

# 引数に espeak_path を追加したよ！
def generate_audio(text, lang_code, voice_name, espeak_path=""):
    global _ort_session, _vocab, _g2p_ja, _g2p_en, _phonemizer
    try:
        setup_espeak_path(espeak_path)
        load_tts_resources()
        if _ort_session is None or _vocab is None: return []

        raw_chunks = re.split(r'(?<=[。！？.!?\n])', text)
        processed_chunks = []
        current_chunk = ""

        for chunk in raw_chunks:
            if len(current_chunk) + len(chunk) > 400:
                processed_chunks.append(current_chunk)
                current_chunk = chunk
            else:
                current_chunk += chunk
        if current_chunk:
            processed_chunks.append(current_chunk)

        full_audio = []

        for chunk in processed_chunks:
            chunk = chunk.strip()
            if not chunk: continue

            phonemes = ""
            if lang_code == "ja" and _g2p_ja is not None:
                phonemes, _ = _g2p_ja(chunk)
            elif (lang_code == "en-us" or lang_code == "en-gb") and _g2p_en is not None:
                phonemes, _ = _g2p_en(chunk)
            elif _phonemizer is not None:
                try:
                    espeak_lang = lang_code 
                    if lang_code == "zh": espeak_lang = "cmn" 
                    
                    backend = _phonemizer(language=espeak_lang, preserve_punctuation=True, with_stress=True)
                    phonemes = backend.phonemize([chunk], strip=True)[0]
                except Exception as e:
                    print(f"Phonemizer error for {lang_code}: {e}")
                    phonemes = chunk
            else:
                phonemes = chunk

            tokens = [0]
            for p in phonemes:
                if p in _vocab: tokens.append(_vocab[p])
                elif p.lower() in _vocab: tokens.append(_vocab[p.lower()])
            tokens.append(0)
            
            input_ids = np.array([tokens[:512]], dtype=np.int64)

            voice_path = f"ttsModels/voices/{voice_name}.bin"
            if not os.path.exists(voice_path):
                print(f"Voice file not found: {voice_path}")
                return []

            styles = np.fromfile(voice_path, dtype=np.float32).reshape(-1, 1, 256)
            token_count = min(len(tokens), len(styles) - 1)
            ref_s = styles[token_count]
            
            outputs: typing.Any = _ort_session.run(None, {
                "input_ids": input_ids,
                "style": ref_s,
                "speed": np.array([1.0], dtype=np.float32)
            })
            
            audio_array = np.array(outputs[0]).flatten()
            full_audio.extend(audio_array.tolist())
            full_audio.extend([0.0] * 2400)

        return full_audio
        
    except Exception as e:
        print(f"Raw ONNX TTS Error: {e}")
        return []