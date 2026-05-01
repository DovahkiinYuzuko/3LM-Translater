mod gui;
mod tts_engine;

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    let config_str = std::fs::read_to_string("config.yaml")
        .unwrap_or_else(|_| "{}".to_string());
    let config: gui::AppConfig = serde_yaml::from_str(&config_str)
        .unwrap_or_default();

    let window_size = [config.window_width, config.window_height];

    // assetsフォルダからアイコンを読み込む
    let icon_data = if let Ok(image) = image::open("assets/app-icon.png") {
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
            .with_icon(icon_data.unwrap_or_default()), // アイコン適用
        ..Default::default()
    };

    eframe::run_native(
        "3LMTranslater",
        options,
        Box::new(|cc| Box::new(gui::MyApp::new(cc, config))),
    )
}