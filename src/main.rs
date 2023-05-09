mod ast;
mod eval;
mod typecheck;

use std::borrow::BorrowMut;
use std::cell::RefCell;
use egui::Rgba;
use egui::{Area, Color32, emath, Order, Painter, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use egui::epaint::CubicBezierShape;
use crate::emath::Align2;
use crate::eval::{eval_program, EvalEngine};
use std::default::Default;
use std::rc::Rc;
use egui::Event::PointerButton;
use multimap::MultiMap;
use std::collections::HashMap;

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

#[derive(Clone)]
#[derive(Debug)]
#[derive(Default)]
pub struct Node {
    pub id: usize,
    pub config: NodeConfig,
    pub sub_nodes: Vec<NodeId>,
}

#[derive(Debug)]
#[derive(Default)]
#[derive(Clone)]
pub struct NodeConfig {
    pub pos: Pos2,
    pub size: Vec2,
    pub parent: Option<usize>,
    pub dragged: bool,
    pub resizing: bool,
    pub last_drag_position: Option<Pos2>,
}

type NodeId = usize;
type PortId = usize;

#[derive(Debug)]
struct ProteusApp {
    node_count: usize,
    state: EditorState,
    timer: delta::Timer,
    delta: f64,
    time: f64,
}

#[derive(Debug)]
#[derive(Default)]
struct NodePort {
    id: usize,
    pos: Pos2,
    node: usize,
}

#[derive(Debug)]
#[derive(Default)]
struct EditorState {
    nodes: HashMap<NodeId, Node>,
    node_rects: HashMap<NodeId, Rect>,
    connecting: Option<(NodeId, Pos2)>,
    node_ports: HashMap<PortId, NodePort>,
    connections: Vec<(PortId, PortId)>,
}

impl ProteusApp {
    pub fn new() -> ProteusApp {
        let mut app = ProteusApp {
            node_count: 0,
            state: EditorState::default(),
            timer: delta::Timer::new(),
            delta: 0.0f64,
            time: 0.0,
        };

        app
    }
}

fn draw_port(ui: &mut Ui, port_pos: Pos2, transient: bool) {
    let port_rect = Rect::from_center_size(port_pos, egui::vec2(10.0, 10.0));

    let close_enough = transient && if let Some(pointer_pos) = ui.ctx().pointer_hover_pos() {
        port_rect.center().distance(pointer_pos) < 10.0
    } else {
        false
    };

    let port_color = if close_enough { Color32::WHITE } else { Color32::from_rgb(241,120,41) };
    ui.painter().circle_filled(port_rect.center(), 7.0, port_color);
}

fn draw_connection(ui: &mut Ui, src_pos: Pos2, dst_pos: Pos2, color: Color32) {
    let diff = dst_pos - src_pos;
    let norm_diff = diff.normalized();
    let dist = if diff.length() > 35.0 { 40.0 } else { diff.length() };
    ui.painter().line_segment([ src_pos, dst_pos ], Stroke::new(3.0, color));
    ui.painter().arrow(dst_pos - norm_diff * dist, norm_diff * ((dist - 5.0).max(0.0)), Stroke::new(2.0, color));
}

fn draw_link(ui: &mut Ui, src: &PortId, dst: &PortId, color: Color32, state: &EditorState) {
    let src_node = state.node_ports.get(src).unwrap();
    let dst_node = state.node_ports.get(dst).unwrap();

    let src_pos = src_node.pos + state.node_rects.get(&src_node.node).unwrap().min.to_vec2();
    let dst_pos = dst_node.pos + state.node_rects.get(&dst_node.node).unwrap().min.to_vec2();

    draw_connection(ui, src_pos, dst_pos, color);
    draw_port(ui, src_pos, false);
    draw_port(ui, dst_pos, false);
}

fn find_node_position(node: &Node, editor_state: &EditorState) -> Pos2 {
    let mut pos = node.config.pos;

    if let Some(parent) = node.config.parent {
        pos += find_node_position(editor_state.nodes.get(&parent).unwrap(), editor_state).to_vec2();
    }

    pos
}

fn draw_node(ui: &mut Ui, node_id: NodeId, time: f64, parent_pos: Pos2, node_count: &mut usize, editor_state: &mut EditorState) {

    let mut config = {
        let node = editor_state.nodes.get(&node_id).unwrap();
        node.config.clone()
    };

    let rect = {
        let mut node = editor_state.nodes.get_mut(&node_id).unwrap();
        Rect::from_min_size(parent_pos + node.config.pos.to_vec2(), node.config.size)
    };

    let mut response = ui.allocate_rect(rect,
                                        egui::Sense::click_and_drag()
                                                .union(egui::Sense::hover())
                                                .union(egui::Sense::click()));

    let internal_hover = if let Some(pointer_pos) = ui.ctx().pointer_hover_pos() {
        !config.dragged && !config.resizing &&
            Rect::from_min_size(rect.min + Vec2::new(5.0, 27.0), rect.size() - Vec2::new(5.0, 32.0)).contains(pointer_pos)
    } else { false };

    let edge_hovered = if let Some(pointer_pos) = ui.ctx().pointer_hover_pos() {
        !config.dragged && !config.resizing &&
            Rect::from_min_size(rect.min, Vec2::new(rect.width(), 5.0)).contains(pointer_pos) ||
            rect.expand(2.0).contains(pointer_pos) && !rect.shrink(3.0).contains(pointer_pos)
    } else { false };

    let hovered = if let Some(pointer_pos) = ui.ctx().pointer_hover_pos() {
        config.dragged || config.resizing ||
        Rect::from_min_size(rect.min, Vec2::new(rect.width(), 27.0)).contains(pointer_pos) ||
            rect.expand(2.0).contains(pointer_pos) && !rect.shrink(15.0).contains(pointer_pos)
    } else { false };

    let sub_rect = Rect::from_min_size(config.pos, config.size);

    if editor_state.connecting.is_some() {
        if response.drag_released_by(egui::PointerButton::Primary) {
            if let Some((start, port_pos)) = editor_state.connecting {

                if let Some(pointer_pos) = ui.ctx().pointer_hover_pos() {
                    for (node_id, some_node) in &editor_state.nodes {
                        let window_pos = find_node_position(some_node, editor_state);
                        let node_rect = Rect::from_min_size(window_pos, some_node.config.size);
                        let edge = node_rect.expand(5.0).contains(pointer_pos)
                            && !node_rect.shrink(5.0).contains(pointer_pos);

                        if edge {
                            println!("Started connection on node #{}, ended on node #{}", start, node_id);
                            let start_rect = editor_state.node_rects.get(&start).unwrap();
                            let end_rect = editor_state.node_rects.get(node_id).unwrap();

                            let first_port_id = editor_state.node_ports.len();
                            let start_port = NodePort { id: first_port_id, node: start, pos: port_pos - start_rect.min.to_vec2() };
                            editor_state.node_ports.insert(first_port_id, start_port);

                            let second_port_id = editor_state.node_ports.len();
                            let end_port = NodePort { id: second_port_id, node: *node_id, pos: pointer_pos - end_rect.min.to_vec2() };
                            editor_state.node_ports.insert(second_port_id, end_port);

                            editor_state.connections.push((first_port_id, second_port_id));
                            editor_state.connecting = None;
                            break;
                        } else { println!("Edge not detected"); }
                    }
                } else { println!("No hover position"); }
            } else { println!("Editor not connecting"); }

            editor_state.connecting = None;
        }
    } else if config.dragged {
        if let Some(latest_pos) = ui.ctx().pointer_latest_pos() {
            if let Some(last_drag) = config.last_drag_position {
                let delta = latest_pos - last_drag;
                config.pos += delta;
                config.last_drag_position = Some(latest_pos);
            } else {
                config.last_drag_position = Some(latest_pos);
            }
            editor_state.node_rects.insert(node_id, sub_rect);
        }

        if response.drag_released_by(egui::PointerButton::Primary) {
            config.dragged = false;
            config.last_drag_position = None;
            editor_state.node_rects.insert(node_id, sub_rect);
        }
    }
    else if config.resizing
    {
        if let Some(latest_pos) = ui.ctx().pointer_latest_pos() {
            if let Some(last_drag) = config.last_drag_position {
                let delta = latest_pos - last_drag;
                let old_size = config.size;
                config.size += delta;

                if config.size.x < 100.0 { config.size.x = old_size.x; }
                if config.size.y < 50.0 { config.size.y = old_size.y; }

                config.last_drag_position = Some(latest_pos);
            } else {
                config.last_drag_position = Some(latest_pos);
            }

            editor_state.node_rects.insert(node_id, sub_rect);
        }

        if response.drag_released_by(egui::PointerButton::Primary) {
            config.resizing = false;
            config.last_drag_position = None;
            editor_state.node_rects.insert(node_id, sub_rect);
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
            config.dragged = close_enough_to_move && response.drag_started_by(egui::PointerButton::Primary);
            config.resizing = false;
        } else if close_enough_to_resize && !edge_hovered {
            config.resizing = close_enough_to_resize && response.drag_started_by(egui::PointerButton::Primary);
            config.dragged = false;
        } else if response.drag_started_by(egui::PointerButton::Primary) {
            editor_state.connecting = Option::Some((node_id, response.hover_pos().unwrap()));
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
        if config.resizing {
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
                 format!("Node #{}", node_id), egui::FontId::new(14.0,egui::FontFamily::Proportional),
                      egui::Color32::GRAY,);

    if !config.dragged && !config.resizing && edge_hovered {
        if let Some(pointer_pos) = ui.ctx().pointer_hover_pos() {
            draw_port(ui, pointer_pos, true);
        }
    } else if internal_hover {
        if response.double_clicked_by(egui::PointerButton::Primary) {
            *node_count += 1;
            let next_pos = (response.hover_pos().unwrap() - parent_pos - config.pos.to_vec2()).to_pos2();
            let next_size = Vec2::new(200.0, 150.0);
            let id = *node_count;
            let sub_node = Node {
                id: *node_count,
                config: NodeConfig {
                    pos: next_pos,
                    size: next_size,
                    parent: Some(node_id),
                    ..Default::default()
                }, sub_nodes: vec![] };

            editor_state.nodes.insert(id, sub_node);
            editor_state.nodes.get_mut(&node_id).unwrap().sub_nodes.push(id);
            editor_state.node_rects.insert(id, Rect::from_min_size(next_pos, next_size));
        }
    }

    for sub in editor_state.nodes.get(&node_id).unwrap().sub_nodes.clone() {
        draw_node(ui, sub, time, parent_pos + config.pos.to_vec2(), node_count, editor_state);
    }

    if let Some((_, pos)) = editor_state.connecting {
        let end_point = ui.ctx().pointer_hover_pos().unwrap();
        draw_port(ui, pos, true);
        draw_connection(ui, pos, end_point, Color32::WHITE);
        draw_port(ui, end_point, true);
    }

    editor_state.nodes.get_mut(&node_id).unwrap().config = config;
}

impl eframe::App for ProteusApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.delta = self.timer.mark_millis() as f64 / 1000.0;
        self.time += self.delta;

        egui::CentralPanel::default().show(ctx, |ui| {
            let response = ui.allocate_response(ui.available_size(), Sense::click());
            if response.double_clicked()
            {
                if let Some(pos) = response.hover_pos() {
                    self.node_count += 1;
                    let id = self.node_count;
                    let size = Vec2::new(300.0, 250.0);
                    let sub_node = Node {
                        id: self.node_count,
                        config: NodeConfig {
                            pos,
                            size,
                            ..Default::default()
                        },
                        sub_nodes: vec![]
                    };

                    self.state.nodes.insert(id, sub_node);
                    self.state.node_rects.insert(id, Rect::from_min_size(pos, size));
                }
            }

            ui.painter().rect_filled(ctx.screen_rect(), 0.0, Color32::BLACK);

            let nodes: Vec<_> = self.state.nodes.clone().into_keys().collect();
            for id in &nodes {
                if self.state.nodes.get(id).unwrap().config.parent.is_none() {
                    draw_node(ui, *id, self.time, Pos2::ZERO, &mut self.node_count, &mut self.state);
                }
            }

            for (start, end) in &self.state.connections {
                draw_link(ui, start, end, Color32::WHITE, &self.state);
            }
        });
    }
}