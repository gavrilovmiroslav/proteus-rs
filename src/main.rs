mod ast;
mod eval;
mod typecheck;

use egui::Rgba;
use egui::{Area, Color32, emath, Order, Painter, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use egui::epaint::CubicBezierShape;
use crate::emath::Align2;
use crate::eval::{eval_program, EvalEngine};
use std::default::Default;

fn main() {
    let mut engine = EvalEngine::default();

    engine.load_from_file("tests/lights.pro").expect("Program loading failed");
    assert!(engine.compile().is_ok());
    println!("{:#?}", engine);


    let native_options = eframe::NativeOptions {
        initial_window_size: Some(eframe::egui::vec2(1600., 800.)),
        ..Default::default()
    };

    eframe::run_native("My egui App", native_options, Box::new(|cc| Box::new(ProteusApp::new())));
}

#[derive(Debug)]
#[derive(Default)]
pub struct Node {
    pub config: NodeConfig,
    pub sub_nodes: Vec<Node>,
}

#[derive(Debug)]
#[derive(Default)]
pub struct NodeConfig {
    pub pos: Pos2,
    pub size: Vec2,
    pub dragged: bool,
    pub resizing: bool,
    pub last_drag_position: Option<Pos2>,
}

#[derive(Debug)]
struct ProteusApp {
    node: Node,
    timer: delta::Timer,
    delta: f64,
    time: f64,
}

impl ProteusApp {
    pub fn new() -> ProteusApp {
        let mut app = ProteusApp {
            timer: delta::Timer::new(),
            delta: 0.0f64,
            node: Node {
                config: NodeConfig {
                    pos: Pos2::new(300.0, 300.0),
                    size: Vec2::new(300.0, 250.0),
                    ..Default::default()
                },
                sub_nodes: vec![]
            },
            time: 0.0,
        };

        app
    }
}

fn draw_port(ui: &mut Ui, port_pos: Pos2, transient: bool, time: f64) {
    let port_rect = Rect::from_center_size(port_pos, egui::vec2(10.0, 10.0));

    let close_enough = transient && if let Some(pointer_pos) = ui.ctx().pointer_hover_pos() {
        port_rect.center().distance(pointer_pos) < 10.0
    } else {
        false
    };

    let port_color = if close_enough { Color32::WHITE } else { Color32::from_rgb(241,120,41) };
    ui.painter().circle_filled(port_rect.center(), 7.0, port_color);
}

fn draw_connection(painter: &Painter, src_pos: Pos2, dst_pos: Pos2, color: Color32) {
    let connection_stroke = egui::Stroke { width: 1.0, color };

    let control_scale = ((dst_pos.x - src_pos.x) / 2.0).max(30.0);
    let src_control = src_pos + Vec2::X * control_scale;
    let dst_control = dst_pos - Vec2::X * control_scale;

    let bezier = CubicBezierShape::from_points_stroke(
        [src_pos, src_control, dst_control, dst_pos],
        false,
        Color32::TRANSPARENT,
        connection_stroke,
    );

    painter.add(bezier);
}

fn draw_node(ui: &mut Ui, node: &mut Node, time: f64, parent_pos: Pos2) {
    let rect = Rect::from_min_size(parent_pos + node.config.pos.to_vec2(), node.config.size);

    let mut response = ui.allocate_rect(rect,
                                        egui::Sense::click_and_drag()
                                                .union(egui::Sense::hover())
                                                .union(egui::Sense::click()));

    let edge_hovered = if let Some(pointer_pos) = ui.ctx().pointer_hover_pos() {
        !node.config.dragged && !node.config.resizing &&
            Rect::from_min_size(rect.min, Vec2::new(rect.width(), 5.0)).contains(pointer_pos) ||
            rect.expand(2.0).contains(pointer_pos) && !rect.shrink(3.0).contains(pointer_pos)
    } else { false };

    let hovered = if let Some(pointer_pos) = ui.ctx().pointer_hover_pos() {
        node.config.dragged || node.config.resizing ||
        Rect::from_min_size(rect.min, Vec2::new(rect.width(), 27.0)).contains(pointer_pos) ||
            rect.expand(2.0).contains(pointer_pos) && !rect.shrink(15.0).contains(pointer_pos)
    } else { false };

    if node.config.dragged
    {
        if let Some(latest_pos) = ui.ctx().pointer_latest_pos() {
            if let Some(last_drag) = node.config.last_drag_position {
                let delta = latest_pos - last_drag;
                node.config.pos += delta;
                node.config.last_drag_position = Some(latest_pos);
            } else {
                node.config.last_drag_position = Some(latest_pos);
            }
        }

        if response.drag_released_by(egui::PointerButton::Primary) {
            node.config.dragged = false;
            node.config.last_drag_position = None;
        }
    }
    else if node.config.resizing
    {
        if let Some(latest_pos) = ui.ctx().pointer_latest_pos() {
            if let Some(last_drag) = node.config.last_drag_position {
                let delta = latest_pos - last_drag;
                let old_size = node.config.size;
                node.config.size += delta;

                if node.config.size.x < 100.0 { node.config.size.x = old_size.x; }
                if node.config.size.y < 50.0 { node.config.size.y = old_size.y; }

                node.config.last_drag_position = Some(latest_pos);
            } else {
                node.config.last_drag_position = Some(latest_pos);
            }
        }

        if response.drag_released_by(egui::PointerButton::Primary) {
            node.config.resizing = false;
            node.config.last_drag_position = None;
        }
    } else {
        let close_enough_to_resize = if let Some(pointer_pos) = ui.ctx().pointer_hover_pos() {
            Rect::from_min_size(rect.max - Vec2::new(20.0, 20.0), Vec2::new(20.0, 20.0)).contains(pointer_pos)
        } else { false };

        let close_enough_to_move = if let Some(pointer_pos) = ui.ctx().pointer_hover_pos() {
            Rect::from_min_size(rect.min, Vec2::new(rect.width(), 27.0)).contains(pointer_pos) ||
                rect.expand(2.0).contains(pointer_pos) && !rect.shrink(10.0).contains(pointer_pos)
        } else {
            false
        };

        if !close_enough_to_resize && !edge_hovered {
            node.config.dragged = close_enough_to_move && response.drag_started_by(egui::PointerButton::Primary);
            node.config.resizing = false;
        } else if close_enough_to_resize && !edge_hovered {
            node.config.resizing = close_enough_to_resize && response.drag_started_by(egui::PointerButton::Primary);
            node.config.dragged = false;
        } else {
            // connection
        }
    }

    let stroke = if hovered {
        egui::Stroke::new(1.0, egui::Color32::WHITE)
    } else {
        egui::Stroke::new(1.0, egui::Color32::DARK_GRAY)
    };

    ui.painter().rect(
        rect.shrink(if hovered { 0.0 } else { 1.0 }),
        3.0,
        ui.ctx().style().visuals.window_fill(),
        stroke);

    ui.painter().rect_filled(
        Rect::from_min_size(rect.min, Vec2::new(rect.width(), 25.0)).shrink(1.0),
        0.0,
        ui.ctx().style().visuals.code_bg_color);

    ui.painter().line_segment(
        [
            rect.left_top() + Vec2::new(1.0, 25.0),
            rect.right_top() + Vec2::new(-1.0, 25.0),
        ],
        stroke);

    /* resize gizmo */ {
        let gizmo_pos = rect.right_bottom() - Vec2::new(10.0, 10.0);
        if node.config.resizing {
            ui.painter().circle_filled(gizmo_pos, 5.0, Color32::DARK_GRAY);
        } else {
            let mut painted = false;
            if let Some(pointer_pos) = ui.ctx().pointer_hover_pos() {
                if pointer_pos.distance(gizmo_pos) <= 5.0 {
                    ui.painter().circle_filled(gizmo_pos, 5.0, Color32::DARK_GRAY.gamma_multiply(0.5));
                    painted = true;
                }
            }

            if !painted {
                ui.painter().circle_stroke(gizmo_pos, 5.0, Stroke::new(1.0, Color32::DARK_GRAY));
            }
        }
    }

    ui.painter().text(rect.left_top() + Vec2::new(10.0, 5.0), Align2::LEFT_TOP,
                 "Node Name", egui::FontId::new(14.0,egui::FontFamily::Proportional),
                      egui::Color32::GRAY,);

    if !node.config.dragged && !node.config.resizing && edge_hovered {
        if let Some(pointer_pos) = ui.ctx().pointer_hover_pos() {
            draw_port(ui, pointer_pos, true, time);
        }
    } else if !node.config.dragged && !node.config.resizing && !edge_hovered {
        if response.double_clicked_by(egui::PointerButton::Primary) {
            node.sub_nodes.push(Node { config: NodeConfig {
                pos: response.hover_pos().unwrap(),
                size: Vec2::new(200.0, 150.0),
                dragged: false,
                resizing: false,
                last_drag_position: None
            }, sub_nodes: vec![] })
        }
    }

    for sub in &mut node.sub_nodes {
        draw_node(ui, sub, time, parent_pos + node.config.pos.to_vec2());
    }
}

impl eframe::App for ProteusApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.delta = self.timer.mark_millis() as f64 / 1000.0;
        self.time += self.delta;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.painter().rect_filled(ctx.screen_rect(), 0.0, Color32::BLACK);
            draw_node(ui, &mut self.node, self.time, Pos2::ZERO);
        });
    }
}