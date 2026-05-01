mod gui;
mod tts_engine;

use eframe::egui;
use std::path::PathBuf;

// どこから起動してもexeの場所を基準にする関数
pub fn get_resource_path(relative: &str) -> PathBuf {
    let mut exe_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    exe_path.pop();
    exe_path.join(relative)
}

fn main() -> Result<(), eframe::Error> {
    // 【超重要】カレントディレクトリをexeの場所に強制変更
    // これでPython側の「ttsModels/〜」みたいな相対パスも全部解決する
    if let Ok(mut exe_path) = std::env::current_exe() {
        exe_path.pop();
        let _ = std::env::set_current_dir(exe_path);
    }

    let config_path = get_resource_path("config.yaml");
    let config_str = std::fs::read_to_string(&config_path)
        .unwrap_or_else(|_| "{}".to_string());
    let config: gui::AppConfig = serde_yaml::from_str(&config_str)
        .unwrap_or_default();

    let window_size = [config.window_width, config.window_height];

    // アイコンはバイナリに焼き込む（これでassetsフォルダを配らなくてよくなる）
    let icon_data = if let Ok(image) = image::load_from_memory(include_bytes!("../assets/app-icon.png")) {
        let image = image.to_rgba8();
        let (width, height) = image.dimensions();
        Some(egui::IconData {
            rgba: image.into_raw(),
            width,
            height,
        })
    } else {
        None
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(window_size)
            .with_icon(icon_data.unwrap_or_default()),
        ..Default::default()
    };

    eframe::run_native(
        "3LMTranslater",
        options,
        Box::new(|cc| Box::new(gui::MyApp::new(cc, config))),
    )
}