mod ast;
mod eval;
mod typecheck;

use crate::eval::{eval_program, EvalEngine};

fn main() {
    let mut engine = EvalEngine::default();

    engine.load_from_file("tests/lights.pro").expect("Program loading failed");
    assert!(engine.compile().is_ok());
    println!("{:#?}", engine);



/*    let native_options = eframe::NativeOptions {
        initial_window_size: Some(eframe::egui::vec2(1600., 800.)),
        ..Default::default()
    };

    eframe::run_native("My egui App", native_options, Box::new(|cc| Box::new(MyEguiApp::new(cc))));*/

}

#[derive(Default)]
struct MyEguiApp {
    pub windows: Vec<egui::Window<'static>>,
}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        app.windows.push(egui::Window::new("w1"));
        app
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {

        });
    }
}