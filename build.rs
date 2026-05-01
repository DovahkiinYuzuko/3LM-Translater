use std::path::Path;
// image::imageops::FilterType を使うためにインポートを追加
use image::imageops::FilterType;

fn main() {
    let png_path = "assets/app-icon.png";
    let ico_path = "assets/icon.ico";

    // PNGがあってICOがない場合、またはPNGの方が新しい場合に自動変換
    if Path::new(png_path).exists() {
        let img = image::open(png_path).expect("Failed to open app-icon.png");
        
        // ICOは最大256x256までなので、リサイズ処理を追加！
        let resized_img = img.resize_exact(256, 256, FilterType::Lanczos3);
        
        // リサイズした画像をICOとして保存
        resized_img.save(ico_path).expect("Failed to save icon.ico");
    }

    // Windowsの場合のみ実行ファイルにアイコンを焼き込む
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winres::WindowsResource::new();
        if Path::new(ico_path).exists() {
            res.set_icon(ico_path);
        }
        res.compile().expect("Failed to compile Windows resources");
    }
}