#![allow(unsafe_op_in_unsafe_fn)] 

use eframe::egui;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use rfd::FileDialog;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AppConfig {
    #[serde(default = "default_ui_lang")]
    pub ui_lang: String,
    
    #[serde(default = "default_window_width")]
    pub window_width: f32,
    #[serde(default = "default_window_height")]
    pub window_height: f32,

    pub model_path: String,
    pub device: String,
    
    #[serde(default)]
    pub espeak_path: String,

    pub languages: Vec<String>,
    pub default_src: String,
    pub default_target: String,
    pub max_new_tokens: i32,
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: i32,
    pub repeat_penalty: f32,
    #[serde(default)]
    pub system_prompt: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ui_lang: default_ui_lang(),
            window_width: default_window_width(),
            window_height: default_window_height(),
            model_path: "".to_string(),
            device: "auto".to_string(),
            espeak_path: "".to_string(),
            languages: vec![],
            default_src: "".to_string(),
            default_target: "".to_string(),
            max_new_tokens: 512,
            temperature: 0.1,
            top_p: 0.9,
            top_k: 40,
            repeat_penalty: 1.1,
            system_prompt: "".to_string(),
        }
    }
}

fn default_ui_lang() -> String { "Japanese".to_string() }
fn default_window_width() -> f32 { 800.0 }
fn default_window_height() -> f32 { 700.0 }

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
struct I18nLabels {
    pub ui_lang_select: String,
    pub settings_panel: String,
    pub model_select: String,
    pub open_folder: String,
    pub unselected: String,
    pub force_rebuild: String,
    pub params_adjust: String,
    pub sys_prompt: String,
    pub sys_prompt_desc: String,
    pub warning_no_model: String,
    pub loading_model: String,
    pub ready: String,
    pub translating: String,
    pub clear_btn: String,
    pub copy_btn: String,
    pub translate_btn: String,
    pub disclaimer: String,
    pub error_prefix: String,
    pub playing_audio: String,
    pub current_model: String,
    pub system_log: String,
    pub log_app_started: String,
    pub log_setup_started: String,
    pub log_model_loading: String,
    pub log_model_loaded: String,
    pub log_translation_started: String,
    pub log_translation_complete: String,
    pub log_tts_kokoro: String,
    pub log_tts_os: String,
    pub log_copied: String,
}

impl Default for I18nLabels {
    fn default() -> Self {
        get_fallback_labels()
    }
}

fn get_fallback_labels() -> I18nLabels {
    I18nLabels {
        ui_lang_select: "UI言語".to_string(),
        settings_panel: "制御パネル".to_string(),
        model_select: "モデル選択:".to_string(),
        open_folder: "フォルダを開く (.gguf)".to_string(),
        unselected: "未選択".to_string(),
        force_rebuild: "エンジン強制再構築".to_string(),
        params_adjust: "パラメータ調整".to_string(),
        sys_prompt: "システムプロンプト".to_string(),
        sys_prompt_desc: "翻訳のルールをAIに指示します".to_string(),
        warning_no_model: "注意 モデルを選んでください".to_string(),
        loading_model: "待機 読み込み中...".to_string(),
        ready: "完了 準備完了".to_string(),
        translating: "⚙ 翻訳中...".to_string(),
        clear_btn: "クリア".to_string(),
        copy_btn: "コピー".to_string(),
        translate_btn: "  Translate  ".to_string(),
        disclaimer: "※AI翻訳です。AIは間違える可能性しか有りません。※".to_string(),
        error_prefix: "エラー: ".to_string(),
        playing_audio: "🔊 再生中...".to_string(),
        current_model: "稼働中のモデル:".to_string(),
        system_log: "システムログ".to_string(),
        log_app_started: "[System] アプリケーションを起動しました。".to_string(),
        log_setup_started: "推論エンジンのセットアップを開始しました...".to_string(),
        log_model_loading: "モデルをロードしています...".to_string(),
        log_model_loaded: "モデルのロードが完了しました。".to_string(),
        log_translation_started: "翻訳プロセスを開始しました。".to_string(),
        log_translation_complete: "翻訳プロセスが正常に完了しました。".to_string(),
        log_tts_kokoro: "KokoroTTSで音声を生成・再生します。".to_string(),
        log_tts_os: "OS標準TTSで音声を再生します (OSの設定で音声パック等が必要です)。".to_string(),
        log_copied: "出力テキストをクリップボードにコピーしました。".to_string(),
    }
}

enum WorkerMessage {
    DependencyCheckResult(bool),
    SetupFinished(Result<(), String>),
    ModelLoaded(Result<(), String>),
    StreamChunk(String),
    TranslationDone(Result<String, String>),
}

#[pyclass]
struct StreamCallback {
    tx: Sender<WorkerMessage>,
}

#[pymethods]
impl StreamCallback {
    #[pyo3(signature = (chunk,))]
    fn __call__(&self, chunk: String) {
        let _ = self.tx.send(WorkerMessage::StreamChunk(chunk));
    }
}

#[derive(PartialEq)]
enum AppState {
    CheckingDependencies,
    SetupRequired,
    SettingUp,
    NoModel,
    LoadingModel,
    Ready,
    Translating,
    Error(String),
}

pub struct MyApp {
    pub config: AppConfig,
    i18n_all: BTreeMap<String, I18nLabels>,
    input_text: String,
    output_text: String,
    raw_translation_buffer: String,
    logs: Vec<String>,
    src_lang: String,
    target_lang: String,
    state: AppState,
    tx: Sender<WorkerMessage>,
    rx: Receiver<WorkerMessage>,
    is_speaking: Arc<AtomicBool>,
}

impl MyApp {
    pub fn new(cc: &eframe::CreationContext<'_>, mut config: AppConfig) -> Self {
        setup_custom_fonts(&cc.egui_ctx);
        
        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = egui::Color32::from_rgb(25, 25, 25);
        visuals.panel_fill = egui::Color32::from_rgb(20, 20, 20);
        visuals.override_text_color = Some(egui::Color32::from_rgb(230, 230, 230));
        cc.egui_ctx.set_visuals(visuals);

        let mut style = (*cc.egui_ctx.style()).clone();
        style.text_styles = [
            (egui::TextStyle::Heading, egui::FontId::new(20.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Body, egui::FontId::new(15.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Monospace, egui::FontId::new(15.0, egui::FontFamily::Monospace)),
            (egui::TextStyle::Button, egui::FontId::new(15.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Small, egui::FontId::new(12.0, egui::FontFamily::Proportional)),
        ].into();
        cc.egui_ctx.set_style(style);

        let (tx, rx) = channel();
        
        let i18n_all = match std::fs::read_to_string("i18n.yaml") {
            Ok(content) => serde_yaml::from_str(&content).unwrap_or_default(),
            Err(_) => BTreeMap::new(),
        };

        if config.system_prompt.is_empty() {
            config.system_prompt = "Translate the following input.\nStrictly follow these linguistic rules:\n1. Do not translate proper nouns or specific titles into their literal English meanings.\n2. Keep the original pronunciation/sound for entities.\n3. Do not add any explanations or extra text.".to_string();
        }
        
        let mut app = Self {
            src_lang: config.default_src.clone(),
            target_lang: config.default_target.clone(),
            config,
            i18n_all,
            input_text: "".to_owned(),
            output_text: "".to_owned(),
            raw_translation_buffer: "".to_owned(),
            logs: vec![],
            state: AppState::CheckingDependencies,
            tx,
            rx,
            is_speaking: Arc::new(AtomicBool::new(false)),
        };

        let init_log = app.current_labels().log_app_started.clone();
        app.add_log(&init_log);

        app.check_dependencies();
        app
    }

    fn add_log(&mut self, message: &str) {
        let time = chrono::Local::now().format("%H:%M:%S");
        self.logs.push(format!("[{}] {}", time, message));
        if self.logs.len() > 50 {
            self.logs.remove(0); 
        }
    }

    fn current_labels(&self) -> I18nLabels {
        self.i18n_all.get(&self.config.ui_lang).cloned().unwrap_or_else(get_fallback_labels)
    }

    fn save_config(&self) {
        if let Ok(yaml) = serde_yaml::to_string(&self.config) {
            let _ = std::fs::write("config.yaml", yaml);
        }
    }

    fn check_dependencies(&self) {
        let tx_clone = self.tx.clone();
        thread::spawn(move || {
            pyo3::prepare_freethreaded_python();
            let has_llama = Python::with_gil(|py| py.import("llama_cpp").is_ok());
            let _ = tx_clone.send(WorkerMessage::DependencyCheckResult(has_llama));
        });
    }

    fn run_setup_wizard(&mut self) {
        self.state = AppState::SettingUp;
        self.add_log(&self.current_labels().log_setup_started);
        let tx_clone = self.tx.clone();
        
        thread::spawn(move || {
            let mut cmd = std::process::Command::new("python");
            cmd.arg("./python_bridge/setup_wizard.py");
            
            match cmd.output() {
                Ok(output) => {
                    if output.status.success() {
                        let _ = tx_clone.send(WorkerMessage::SetupFinished(Ok(())));
                    } else {
                        let err = String::from_utf8_lossy(&output.stderr).to_string();
                        let _ = tx_clone.send(WorkerMessage::SetupFinished(Err(err)));
                    }
                }
                Err(e) => {
                    let _ = tx_clone.send(WorkerMessage::SetupFinished(Err(e.to_string())));
                }
            }
        });
    }

    fn load_model(&mut self, absolute_path: String) {
        self.state = AppState::LoadingModel;
        self.add_log(&self.current_labels().log_model_loading);
        let tx_clone = self.tx.clone();
        
        thread::spawn(move || {
            pyo3::prepare_freethreaded_python();
            let res = Python::with_gil(|py| -> PyResult<()> {
                let sys = py.import("sys")?;
                sys.getattr("path")?.call_method1("append", ("./python_bridge",))?;
                let engine = py.import("engine")?;
                
                engine.call_method1("load_model", (absolute_path,))?;
                Ok(())
            });

            match res {
                Ok(_) => { 
                    let _ = tx_clone.send(WorkerMessage::ModelLoaded(Ok(()))); 
                }
                Err(e) => { 
                    let _ = tx_clone.send(WorkerMessage::ModelLoaded(Err(e.to_string()))); 
                }
            }
        });
    }

    fn start_translation(&mut self) {
        self.state = AppState::Translating;
        self.output_text.clear();
        self.raw_translation_buffer.clear();
        self.add_log(&self.current_labels().log_translation_started);

        let tx_clone = self.tx.clone();
        let text = self.input_text.clone();
        let config = self.config.clone();
        let src = self.src_lang.clone();
        let target = self.target_lang.clone();

        thread::spawn(move || {
            pyo3::prepare_freethreaded_python();
            let res = Python::with_gil(|py| -> PyResult<String> {
                let engine = py.import("engine")?;
                let params = PyDict::new(py);
                params.set_item("max_new_tokens", config.max_new_tokens)?;
                params.set_item("temperature", config.temperature)?;
                params.set_item("top_p", config.top_p)?;
                params.set_item("top_k", config.top_k)?;
                params.set_item("repeat_penalty", config.repeat_penalty)?;
                params.set_item("src_lang", src)?;
                params.set_item("target_lang", target)?;
                params.set_item("system_prompt", config.system_prompt)?;

                let callback = Py::new(py, StreamCallback { tx: tx_clone.clone() })?;
                params.set_item("callback", callback)?;

                let result: String = engine.call_method1("translate", (text, params))?.extract()?;
                Ok(result)
            });

            match res {
                Ok(translated) => { 
                    let _ = tx_clone.send(WorkerMessage::TranslationDone(Ok(translated))); 
                }
                Err(e) => { 
                    let _ = tx_clone.send(WorkerMessage::TranslationDone(Err(e.to_string()))); 
                }
            }
        });
    }

    fn show_setup_ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.heading("3LMTranslater - セットアップ");
            ui.add_space(20.0);
            ui.label("推論エンジン (llama-cpp-python) の自動構築を開始します。");
            
            ui.add_space(30.0);

            if self.state == AppState::SettingUp {
                ui.spinner();
                ui.add_space(10.0);
                ui.colored_label(egui::Color32::from_rgb(0, 150, 255), "インストール中です...");
            } else {
                if ui.button("最適化インストールを開始").clicked() {
                    self.run_setup_wizard();
                }
            }
        });
    }

    fn show_main_ui(&mut self, ctx: &egui::Context) {
        let labels = self.current_labels();
        let speaking = self.is_speaking.load(Ordering::SeqCst);

        egui::SidePanel::right("settings_panel")
            .resizable(true)
            .min_width(240.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(10.0);
                    
                    ui.label(egui::RichText::new(&labels.settings_panel).heading().color(egui::Color32::from_rgb(150, 200, 255)));
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.label(&labels.ui_lang_select);
                        let mut ui_lang_changed = false;
                        
                        let mut available_langs: Vec<_> = self.i18n_all.keys().cloned().collect();
                        if available_langs.is_empty() { available_langs.push("Japanese".to_string()); }

                        egui::ComboBox::from_id_source("ui_lang_combo")
                            .selected_text(&self.config.ui_lang)
                            .show_ui(ui, |ui| {
                                for lang in available_langs {
                                    if ui.selectable_value(&mut self.config.ui_lang, lang.clone(), lang).changed() {
                                        ui_lang_changed = true;
                                    }
                                }
                            });
                        if ui_lang_changed {
                            self.save_config();
                        }
                    });

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);

                    ui.label(&labels.model_select);
                    if ui.button(&labels.open_folder).clicked() {
                        if let Some(path) = FileDialog::new().add_filter("GGUF", &["gguf"]).pick_file() {
                            let path_str = path.display().to_string();
                            self.config.model_path = path_str.clone();
                            self.save_config();
                            self.load_model(path_str);
                        }
                    }
                    
                    let path_label = if self.config.model_path.is_empty() {
                        labels.unselected.clone()
                    } else {
                        let path = std::path::Path::new(&self.config.model_path);
                        path.file_name().unwrap_or_default().to_string_lossy().to_string()
                    };
                    ui.label(egui::RichText::new(path_label).small().color(egui::Color32::from_rgb(150, 150, 150)));

                    ui.add_space(10.0);
                    if ui.button(&labels.force_rebuild).clicked() {
                        self.state = AppState::SetupRequired;
                    }

                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(10.0);

                    ui.label(egui::RichText::new(&labels.params_adjust).heading().color(egui::Color32::from_rgb(150, 200, 255)));
                    let mut changed = false;
                    
                    changed |= ui.add(egui::Slider::new(&mut self.config.max_new_tokens, 64..=4096).text("Max Tokens")).changed();
                    changed |= ui.add(egui::Slider::new(&mut self.config.temperature, 0.0..=2.0).text("Temp")).changed();
                    changed |= ui.add(egui::Slider::new(&mut self.config.top_p, 0.0..=1.0).text("Top P")).changed();
                    changed |= ui.add(egui::Slider::new(&mut self.config.top_k, 1..=100).text("Top K")).changed();
                    changed |= ui.add(egui::Slider::new(&mut self.config.repeat_penalty, 1.0..=2.0).text("Penalty")).changed();
                    
                    if changed {
                        self.save_config();
                    }

                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(10.0);

                    ui.label(egui::RichText::new(&labels.sys_prompt).heading().color(egui::Color32::from_rgb(150, 200, 255)));
                    ui.label(egui::RichText::new(&labels.sys_prompt_desc).small().color(egui::Color32::from_rgb(150, 150, 150)));
                    
                    egui::Frame::none().inner_margin(4.0).show(ui, |ui| {
                        let prompt_response = ui.add(
                            egui::TextEdit::multiline(&mut self.config.system_prompt)
                                .desired_width(ui.available_width())
                                .desired_rows(6)
                                .text_color(egui::Color32::from_rgb(230, 230, 230))
                        );
                        if prompt_response.changed() {
                            self.save_config();
                        }
                    });

                    ui.add_space(20.0);
                });
            });

        egui::TopBottomPanel::bottom("footer_panel").show(ctx, |ui| {
            egui::Frame::none().inner_margin(10.0).show(ui, |ui| {
                egui::CollapsingHeader::new(if self.i18n_all.is_empty() { "System Log" } else { &labels.system_log })
                    .default_open(false)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(100.0)
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                for log in &self.logs {
                                    ui.label(egui::RichText::new(log).small().color(egui::Color32::from_rgb(180, 180, 180)));
                                }
                            });
                    });

                ui.add_space(5.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new(&labels.disclaimer).small().color(egui::Color32::from_rgb(100, 100, 100)));
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::none().inner_margin(20.0).show(ui, |ui| {
                ui.label(egui::RichText::new("3LMTranslater").heading().size(28.0).color(egui::Color32::from_rgb(200, 220, 255)));
                
                let current_model_name = if self.config.model_path.is_empty() {
                    labels.unselected.clone()
                } else {
                    std::path::Path::new(&self.config.model_path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string()
                };
                ui.label(egui::RichText::new(format!("{} {}", labels.current_model, current_model_name)).color(egui::Color32::from_rgb(180, 255, 180)));

                ui.add_space(5.0);

                match &self.state {
                    AppState::NoModel => { ui.colored_label(egui::Color32::from_rgb(255, 150, 50), &labels.warning_no_model); },
                    AppState::LoadingModel => { ui.colored_label(egui::Color32::from_rgb(255, 150, 50), &labels.loading_model); },
                    AppState::Ready => { ui.colored_label(egui::Color32::from_rgb(50, 255, 100), &labels.ready); },
                    AppState::Translating => { ui.colored_label(egui::Color32::from_rgb(100, 200, 255), &labels.translating); },
                    AppState::Error(e) => { ui.colored_label(egui::Color32::from_rgb(255, 50, 50), format!("{}{}", labels.error_prefix, e)); },
                    _ => {}
                };

                ui.add_space(10.0);
                
                let mut lang_changed = false;
                ui.horizontal(|ui| {
                    ui.label("From:");
                    egui::ComboBox::from_id_source("src_combo").selected_text(&self.src_lang).show_ui(ui, |ui| {
                        for lang in &self.config.languages {
                            if ui.selectable_value(&mut self.src_lang, lang.clone(), lang).changed() { lang_changed = true; }
                        }
                    });

                    ui.label("To:");
                    egui::ComboBox::from_id_source("tgt_combo").selected_text(&self.target_lang).show_ui(ui, |ui| {
                        for lang in &self.config.languages {
                            if ui.selectable_value(&mut self.target_lang, lang.clone(), lang).changed() { lang_changed = true; }
                        }
                    });
                });

                if lang_changed {
                    self.config.default_src = self.src_lang.clone();
                    self.config.default_target = self.target_lang.clone();
                    self.save_config();
                }

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.heading("Input");
                    if ui.button(&labels.clear_btn).clicked() {
                        self.input_text.clear();
                    }
                });
                
                ui.add(
                    egui::TextEdit::multiline(&mut self.input_text)
                        .desired_width(ui.available_width())
                        .desired_rows(6)
                        .text_color(egui::Color32::from_rgb(230, 230, 230))
                );

                ui.add_space(15.0);
                let is_ready = self.state == AppState::Ready && !speaking;
                if ui.add_enabled(is_ready, egui::Button::new(egui::RichText::new(&labels.translate_btn))).clicked() {
                    self.start_translation();
                }

                ui.add_space(15.0);
                ui.separator();
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    ui.heading("Output");
                    
                    let is_output_empty = self.output_text.trim().is_empty();
                    let is_supported = crate::tts_engine::is_kokoro_supported(&self.target_lang);
                    
                    if speaking {
                        ui.spinner();
                        ui.add_enabled(false, egui::Button::new(&labels.playing_audio));
                    } else {
                        let btn_label = if is_supported { "Play" } else { "Play (OS)" };
                        if ui.add_enabled(!is_output_empty, egui::Button::new(btn_label)).clicked() {
                            if !is_supported {
                                self.add_log(&labels.log_tts_os);
                            } else {
                                self.add_log(&labels.log_tts_kokoro);
                            }
                            
                            let espeak_path_env = self.config.espeak_path.clone();

                            crate::tts_engine::speak(
                                self.output_text.clone(),
                                self.target_lang.clone(),
                                Arc::clone(&self.is_speaking),
                                ctx.clone(),
                                espeak_path_env 
                            );
                        }
                    }

                    if ui.button(&labels.copy_btn).clicked() {
                        ui.output_mut(|o| o.copied_text = self.output_text.clone());
                        self.add_log(&labels.log_copied);
                    }
                    if ui.button(&labels.clear_btn).clicked() {
                        self.output_text.clear();
                    }
                });
                
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.output_text)
                                .desired_width(ui.available_width())
                                .desired_rows(8)
                                .text_color(egui::Color32::from_rgb(230, 230, 230))
                                .interactive(true)
                        );
                    });
            });
        });
    }
}

fn parse_thought_process(text: &str) -> (String, String) {
    let mut text = text.trim();
    
    // 変数使わないから text.contains で存在チェックだけする！
    if text.contains("<|channel>thought") {
        if let Some(end) = text.find("<channel|>") {
            text = text[end + 10..].trim();
        }
    }
    if text.contains("<think>") {
        if let Some(end) = text.find("</think>") {
            text = text[end + 8..].trim();
        } else {
            return (String::new(), String::new());
        }
    }

    if let Some(analysis_start) = text.find("\n\n###") {
        let output = text[..analysis_start].trim().to_string();
        return (String::new(), output);
    }
    if text.starts_with("### ") {
        if let Some(first_newline) = text.find('\n') {
            let output = text[first_newline..].trim().to_string();
            return (String::new(), output);
        }
    }

    (String::new(), text.to_string())
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    
    let font_configs = [
        ("latin", "fonts/NotoSans-VariableFont_wdth,wght.ttf"),
        ("jp", "fonts/NotoSansJP-VariableFont_wght.ttf"),
        ("sc", "fonts/NotoSansSC-Regular.otf"),
        ("kr", "fonts/NotoSansKR-Regular.otf"),
    ];

    let mut loaded_fonts = Vec::new();

    for (name, path) in font_configs {
        if let Ok(font_data) = std::fs::read(path) {
            if font_data.len() > 100_000 {
                fonts.font_data.insert(name.to_owned(), egui::FontData::from_owned(font_data));
                loaded_fonts.push(name.to_owned());
            } else {
                println!("Warning: Skipping invalid or broken font file: {}", path);
            }
        }
    }

    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        if let Some(vec) = fonts.families.get_mut(&family) {
            for name in loaded_fonts.iter().rev() {
                vec.insert(0, name.clone());
            }
        }
    }
    
    ctx.set_fonts(fonts);
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let labels = self.current_labels();

        let current_size = ctx.screen_rect().size();
        if (self.config.window_width - current_size.x).abs() > 1.0 || 
           (self.config.window_height - current_size.y).abs() > 1.0 {
            self.config.window_width = current_size.x;
            self.config.window_height = current_size.y;
            self.save_config();
        }

        if let Ok(msg) = self.rx.try_recv() {
            match msg {
                WorkerMessage::DependencyCheckResult(has_llama) => {
                    if has_llama {
                        self.state = AppState::NoModel;
                        if !self.config.model_path.is_empty() {
                            let path = self.config.model_path.clone();
                            self.load_model(path);
                        }
                    } else {
                        self.state = AppState::SetupRequired;
                    }
                }
                WorkerMessage::SetupFinished(Ok(_)) => {
                    self.state = AppState::NoModel;
                    if !self.config.model_path.is_empty() {
                        let path = self.config.model_path.clone();
                        self.load_model(path);
                    }
                }
                WorkerMessage::SetupFinished(Err(e)) => {
                    self.state = AppState::Error(format!("セットアップ失敗:\n{}", e));
                    self.add_log(&format!("{} {}", labels.error_prefix, e));
                }
                WorkerMessage::ModelLoaded(Ok(_)) => {
                    self.add_log(&labels.log_model_loaded);
                    self.state = AppState::Ready;
                }
                WorkerMessage::ModelLoaded(Err(e)) => {
                    self.state = AppState::Error(format!("ロード失敗: {}", e));
                    self.add_log(&format!("{} {}", labels.error_prefix, e));
                }
                
                WorkerMessage::StreamChunk(chunk) => {
                    self.raw_translation_buffer.push_str(&chunk);
                    let (_, output) = parse_thought_process(&self.raw_translation_buffer);
                    self.output_text = output;
                }
                
                WorkerMessage::TranslationDone(Ok(res)) => {
                    self.add_log(&labels.log_translation_complete);
                    let (_, output) = parse_thought_process(&res);
                    self.output_text = output;
                    self.state = AppState::Ready;
                }
                WorkerMessage::TranslationDone(Err(e)) => {
                    self.add_log(&format!("{} {}", labels.error_prefix, e));
                    self.output_text = e;
                    self.state = AppState::Ready;
                }
            }
        }

        if self.state == AppState::CheckingDependencies {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.spinner();
                    ui.label("環境をチェック中...");
                });
            });
        } else if self.state == AppState::SetupRequired || self.state == AppState::SettingUp {
            egui::CentralPanel::default().show(ctx, |ui| {
                self.show_setup_ui(ui);
            });
        } else {
            self.show_main_ui(ctx);
        }

        if self.state != AppState::Ready && self.state != AppState::NoModel {
            ctx.request_repaint();
        }
    }
}