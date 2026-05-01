use std::thread;
use pyo3::prelude::*;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use eframe::egui::Context;

fn get_kokoro_voice(lang: &str) -> Option<(&'static str, &'static str)> {
    match lang {
        "Japanese" => Some(("jf_nezumi", "ja")),
        "English(UN)" => Some(("bm_lewis", "en-gb")),
        "English(US)" => Some(("af_heart","en-us")),
        "Chinese" | "Chinese (Simplified)" | "Chinese (Traditional)" => Some(("zf_xiaoxiao", "zh")),
        "Spanish" => Some(("ef_dora", "es")),
        "French" => Some(("ff_siwis", "fr")),
        "Italian" => Some(("if_sara", "it")),
        _ => None,
    }
}

pub fn is_kokoro_supported(lang: &str) -> bool {
    get_kokoro_voice(lang).is_some()
}

pub fn speak(text: String, target_lang: String, is_speaking: Arc<AtomicBool>, ctx: Context, espeak_path: String) {
    if text.trim().is_empty() { return; }

    is_speaking.store(true, Ordering::SeqCst);
    let fallback_text = text.clone();

    thread::spawn(move || {
        if let Some((voice_name, lang_code)) = get_kokoro_voice(&target_lang) {
            let mut audio_data: Vec<f32> = Vec::new();
            
            let py_res = Python::with_gil(|py| -> PyResult<()> {
                let sys = py.import("sys")?;
                let bridge_path = crate::get_resource_path("python_bridge").to_string_lossy().to_string();
                sys.getattr("path")?.call_method1("append", (bridge_path,))?;
                let engine = py.import("engine")?;
                
                let result = engine.call_method1("generate_audio", (text, lang_code, voice_name, espeak_path))?;
                audio_data = result.extract()?;
                Ok(())
            });

            if py_res.is_ok() && !audio_data.is_empty() {
                println!("Rust: Audio data received. Size: {} samples", audio_data.len());
                play_audio_data(audio_data);
            } else {
                println!("Kokoro failed, falling back to OS standard.");
                speak_with_os_fallback(&fallback_text);
            }
        } else {
            println!("Target language [{}] not in Kokoro map. Fallback to OS TTS.", target_lang);
            speak_with_os_fallback(&fallback_text);
        }
        
        is_speaking.store(false, Ordering::SeqCst);
        ctx.request_repaint();
    });
}

fn speak_with_os_fallback(text: &str) {
    if let Ok(mut tts) = tts::Tts::default() {
        if let Err(e) = tts.speak(text, true) {
            println!("OS TTS Error: {:?}", e);
        } else {
            println!("Speak with OS TTS finished.");
        }
    } else {
        println!("Failed to initialize OS TTS engine.");
    }
}

fn play_audio_data(audio_data: Vec<f32>) {
    if let Ok((_stream, stream_handle)) = rodio::OutputStream::try_default() {
        if let Ok(sink) = rodio::Sink::try_new(&stream_handle) {
            let buffer = rodio::buffer::SamplesBuffer::new(1, 24000, audio_data);
            sink.append(buffer);
            sink.sleep_until_end();
        }
    }
}