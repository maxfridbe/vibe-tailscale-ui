#![windows_subsystem = "windows"]
mod tailscale;

use clay_layout::layout::{Padding, LayoutAlignmentX, LayoutAlignmentY, Alignment, LayoutDirection};
use clay_layout::elements::{FloatingAttachPointType, FloatingAttachToElement, PointerCaptureMode};
use clay_layout::math::{Vector2, Dimensions};
use clay_layout::{Clay, Declaration, Color, grow, fixed};
use clay_layout::render_commands::{RenderCommandConfig};
use raylib::prelude::*;
use std::sync::{Arc, Mutex};
use std::cell::RefCell;
use tailscale::{TailscaleStatus, NodeInfo};

const FONT_DATA: &[u8] = include_bytes!("../assets/font.ttf");

// Nerd Font Icon Constants
const ICON_LINUX: &str = "\u{f303}";
const ICON_WINDOWS: &str = "\u{f17a}";
const ICON_APPLE: &str = "\u{f179}";
const ICON_ANDROID: &str = "\u{f17b}";
const ICON_EXIT_NODE: &str = "\u{f0ac}";
const ICON_USER: &str = "\u{f007}";
const ICON_COPY: &str = "\u{f0c5}";
const ICON_CIRCLE: &str = "\u{f111}";
const ICON_CIRCLE_OUTLINE: &str = "\u{f10c}";
const ICON_TAILSCALE: &str = "\u{e63d}";
const ICON_SEARCH: &str = "\u{f002}";

struct AppState {
    status: Option<TailscaleStatus>,
    loading: bool,
    error: Option<String>,
    status_msg: Option<String>,
    search_query: String,
    data_changed: bool,
}

struct StringArena {
    strings: RefCell<Vec<Box<str>>>,
}

impl StringArena {
    fn new() -> Self {
        Self { strings: RefCell::new(Vec::with_capacity(100)) }
    }

    fn push(&self, s: String) -> &str {
        let mut strings = self.strings.borrow_mut();
        let sanitized = s.replace('\0', "").into_boxed_str();
        let ptr = sanitized.as_ptr();
        let len = sanitized.len();
        strings.push(sanitized);
        unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len)) }
    }

    fn clear(&self) {
        self.strings.borrow_mut().clear();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state = Arc::new(Mutex::new(AppState {
        status: None,
        loading: true,
        error: None,
        status_msg: None,
        search_query: String::new(),
        data_changed: false,
    }));

    // Periodic status refresh
    let state_periodic = Arc::clone(&state);
    tokio::spawn(async move {
        loop {
            refresh_status(Arc::clone(&state_periodic)).await;
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });

    let (mut rl, thread) = raylib::init()
        .size(1024, 768)
        .title("Tailscale Nodes")
        .resizable()
        .build();

    // Disable Esc as the exit key
    rl.set_exit_key(None);
    rl.set_target_fps(60);

    // Prepare character string including Nerd Font icons
    let mut chars_str: String = (32..127).map(|c| c as u8 as char).collect();
    chars_str.push_str(ICON_LINUX);
    chars_str.push_str(ICON_WINDOWS);
    chars_str.push_str(ICON_APPLE);
    chars_str.push_str(ICON_ANDROID);
    chars_str.push_str(ICON_EXIT_NODE);
    chars_str.push_str(ICON_USER);
    chars_str.push_str(ICON_COPY);
    chars_str.push_str(ICON_CIRCLE);
    chars_str.push_str(ICON_CIRCLE_OUTLINE);
    chars_str.push_str(ICON_TAILSCALE);
    chars_str.push_str(ICON_SEARCH);

    let font = rl.load_font_from_memory(&thread, ".ttf", FONT_DATA, 64, Some(&chars_str))
        .expect("Failed to load font");
    
    let mut clay = Clay::new(Dimensions::new(rl.get_screen_width() as f32, rl.get_screen_height() as f32));
    let arena = StringArena::new();
    let mut font_scale: f32 = 1.0;
    let content_id = unsafe { 
        clay_layout::id::Id { id: clay_layout::bindings::Clay__HashString(clay_layout::bindings::Clay_String::from("content"), 0, 0) }
    };

    clay.set_measure_text_function(|text, config| {
        let size = config.font_size as f32;
        let width = text.len() as f32 * (size * 0.52);
        Dimensions::new(width, size)
    });

    let mut last_input_time = std::time::Instant::now();
    let mut current_target_fps = 60;
    let mut active_notification: Option<(String, std::time::Instant)> = None;

    while !rl.window_should_close() {
        arena.clear();

        // Dynamic FPS logic
        let mut activity = false;
        if rl.get_mouse_delta() != raylib::math::Vector2::zero() || 
           rl.get_mouse_wheel_move() != 0.0 ||
           rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            activity = true;
        }

        // Handle scaling
        if rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) || rl.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL) {
            if rl.is_key_pressed(KeyboardKey::KEY_EQUAL) || rl.is_key_pressed(KeyboardKey::KEY_KP_ADD) {
                font_scale = (font_scale + 0.1).min(3.0);
                activity = true;
            }
            if rl.is_key_pressed(KeyboardKey::KEY_MINUS) || rl.is_key_pressed(KeyboardKey::KEY_KP_SUBTRACT) {
                font_scale = (font_scale - 0.1).max(0.5);
                activity = true;
            }
        }

        // Handle search input
        {
            let mut key_pressed = rl.get_char_pressed();
            if key_pressed.is_some() { activity = true; }
            while let Some(c) = key_pressed {
                if (c as u32) >= 32 && (c as u32) <= 126 {
                    let mut guard = state.lock().unwrap();
                    guard.search_query.push(c);
                    guard.data_changed = true;
                }
                key_pressed = rl.get_char_pressed();
            }
            if rl.is_key_pressed(KeyboardKey::KEY_BACKSPACE) {
                let mut guard = state.lock().unwrap();
                guard.search_query.pop();
                guard.data_changed = true;
                activity = true;
            }
            // Use Escape to clear search
            if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
                let mut guard = state.lock().unwrap();
                guard.search_query.clear();
                guard.data_changed = true;
                activity = true;
            }
        }

        let (current_status, loading, error, status_msg, search_query, data_changed) = {
            let mut guard = state.lock().unwrap();
            let changed = guard.data_changed;
            guard.data_changed = false;
            let msg = guard.status_msg.take();
            (guard.status.clone(), guard.loading, guard.error.clone(), msg, guard.search_query.clone(), changed)
        };

        if let Some(msg) = status_msg {
            active_notification = Some((msg, std::time::Instant::now()));
        }

        if data_changed {
            activity = true;
        }

        // Check if notification should still be visible
        if let Some((_, timestamp)) = active_notification {
            if timestamp.elapsed().as_secs_f32() > 3.0 {
                active_notification = None;
            } else {
                // Keep activity alive while notification is showing
                activity = true; 
            }
        }

        if activity {
            last_input_time = std::time::Instant::now();
        }

        let time_since_input = last_input_time.elapsed().as_secs_f32();
        let new_fps = if !rl.is_window_focused() {
            1
        } else if !rl.is_cursor_on_screen() && time_since_input > 2.0 {
            1
        } else if time_since_input > 5.0 && !loading {
            15
        } else {
            60
        };

        if new_fps != current_target_fps {
            rl.set_target_fps(new_fps);
            current_target_fps = new_fps;
        }

        let mouse_pos = rl.get_mouse_position();
        let mouse_down = rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT);
        let mouse_pressed = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        let scroll_delta = rl.get_mouse_wheel_move_v();
        
        clay.pointer_state(Vector2::new(mouse_pos.x, mouse_pos.y), mouse_down);
        clay.update_scroll_containers(true, Vector2::new(scroll_delta.x * 50.0, scroll_delta.y * 50.0), rl.get_frame_time());
        clay.set_layout_dimensions(Dimensions::new(rl.get_screen_width() as f32, rl.get_screen_height() as f32));

        let mut scroll_pos = Vector2::new(0.0, 0.0);
        if let Some(scroll_data) = clay.scroll_container_data(content_id) {
            scroll_pos = unsafe { (*scroll_data.scrollPosition).into() };
        }

        let mut clay_scope = clay.begin::<Texture2D, ()>();
        
        let mut root_decl = Declaration::new();
        root_decl.id(clay_scope.id("root"))
            .layout()
                .width(grow!())
                .height(grow!())
                .padding(Padding::all(16))
                .child_gap(16)
                .direction(LayoutDirection::TopToBottom)
            .end()
            .background_color(Color::u_rgb(20, 20, 20));

        clay_scope.with(&root_decl, |clay_scope| {
            // Header
            let mut header_decl = Declaration::new();
            header_decl.layout()
                    .width(grow!())
                    .height(fixed!(60.0 * font_scale))
                    .padding(Padding::all(16))
                    .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                    .child_gap(12)
                .end()
                .background_color(Color::u_rgb(40, 40, 40))
                .corner_radius().all(8.0).end();

            clay_scope.with(&header_decl, |clay_scope| {
                clay_scope.text(ICON_TAILSCALE, clay_layout::text::TextConfig::new().font_size((32.0 * font_scale) as u16).color(Color::u_rgb(255, 255, 255)).end());
                clay_scope.text("Tailscale Nodes", clay_layout::text::TextConfig::new().font_size((24.0 * font_scale) as u16).color(Color::u_rgb(255, 255, 255)).end());
            });

            // Search Bar
            let mut search_decl = Declaration::new();
            search_decl.layout()
                    .width(grow!())
                    .height(fixed!(40.0 * font_scale))
                    .padding(Padding::horizontal(12))
                    .child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center))
                    .child_gap(12)
                .end()
                .background_color(Color::u_rgb(35, 35, 35))
                .corner_radius().all(6.0).end();
            
            clay_scope.with(&search_decl, |clay_scope| {
                clay_scope.text(ICON_SEARCH, clay_layout::text::TextConfig::new().font_size((18.0 * font_scale) as u16).color(Color::u_rgb(150, 150, 150)).end());
                let placeholder = if search_query.is_empty() { "Search nodes..." } else { &search_query };
                let color = if search_query.is_empty() { Color::u_rgb(100, 100, 100) } else { Color::u_rgb(255, 255, 255) };
                clay_scope.text(arena.push(placeholder.to_string()), clay_layout::text::TextConfig::new().font_size((18.0 * font_scale) as u16).color(color).end());
            });

            if loading && current_status.is_none() {
                 let mut loading_decl = Declaration::new();
                 loading_decl.layout().width(grow!()).height(grow!()).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end();
                 clay_scope.with(&loading_decl, |clay_scope| {
                    clay_scope.text("Loading...", clay_layout::text::TextConfig::new().font_size((20.0 * font_scale) as u16).color(Color::u_rgb(200, 200, 200)).end());
                 });
            } else if let Some(err) = error {
                 let mut error_decl = Declaration::new();
                 error_decl.layout().width(grow!()).height(grow!()).child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center)).end();
                 let error_msg = arena.push(format!("Error: {}", err));
                 clay_scope.with(&error_decl, |clay_scope| {
                    clay_scope.text(error_msg, clay_layout::text::TextConfig::new().font_size((20.0 * font_scale) as u16).color(Color::u_rgb(255, 100, 100)).end());
                 });
            } else if let Some(s) = current_status {
                let mut content_decl = Declaration::new();
                content_decl.id(content_id)
                    .layout()
                        .width(grow!())
                        .height(grow!())
                        .child_gap(12)
                        .direction(LayoutDirection::TopToBottom)
                        .end()
                    .clip(false, true, scroll_pos);
                
                clay_scope.with(&content_decl, |clay_scope| {
                    let mut all_nodes = vec![(&s.self_node, true)];
                    let mut peers: Vec<(&NodeInfo, bool)> = s.peer.values().map(|p| (p, false)).collect();
                    
                    peers.sort_by(|a, b| {
                        let a_online = a.0.online.unwrap_or(false);
                        let b_online = b.0.online.unwrap_or(false);
                        if a_online != b_online {
                            return b_online.cmp(&a_online);
                        }
                        let a_ls = a.0.last_seen.as_deref().unwrap_or("");
                        let b_ls = b.0.last_seen.as_deref().unwrap_or("");
                        if a_ls != b_ls {
                            return b_ls.cmp(a_ls);
                        }
                        a.0.host_name.cmp(&b.0.host_name)
                    });
                    
                    all_nodes.extend(peers);

                    let search_lower = search_query.to_lowercase();
                    let filtered_nodes: Vec<_> = all_nodes.into_iter().filter(|(n, _)| {
                        search_query.is_empty() || n.host_name.to_lowercase().contains(&search_lower) || n.tailscale_ips.iter().any(|ip| ip.contains(&search_query))
                    }).collect();

                    for (i, (node, is_self)) in filtered_nodes.iter().enumerate() {
                        render_node(clay_scope, node, *is_self, i as u32, mouse_pressed, &state, &arena, font_scale, &mut rl);
                    }
                });
            }

            // Floating Notification Banner
            if let Some((msg, _)) = &active_notification {
                 let mut msg_decl = Declaration::new();
                 msg_decl.layout()
                    .padding(Padding::new(24, 24, 12, 12))
                    .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                    .end()
                    .background_color(Color::u_rgb(0, 80, 150))
                    .corner_radius().all(8.0).end();
                 
                 msg_decl.floating()
                    .offset(Vector2::new(0.0, -32.0))
                    .dimensions(Dimensions::new(0.0, 0.0))
                    .z_index(100)
                    .parent_id(clay_scope.id("root").id.id)
                    .attach_points(FloatingAttachPointType::CenterBottom, FloatingAttachPointType::CenterBottom)
                    .attach_to(FloatingAttachToElement::Parent)
                    .pointer_capture_mode(PointerCaptureMode::Passthrough);

                 let m = arena.push(format!("{} {}", ICON_COPY, msg));
                 clay_scope.with(&msg_decl, |clay_scope| {
                    clay_scope.text(m, clay_layout::text::TextConfig::new().font_size((16.0 * font_scale) as u16).color(Color::u_rgb(255, 255, 255)).end());
                 });
            }
        });

        let render_commands = clay_scope.end();

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(raylib::color::Color::BLACK);
        
        for command in render_commands {
            match command.config {
                RenderCommandConfig::Rectangle(rect) => {
                    if rect.corner_radii.top_left > 0.0 {
                         d.draw_rectangle_rounded(
                            raylib::math::Rectangle::new(command.bounding_box.x, command.bounding_box.y, command.bounding_box.width, command.bounding_box.height),
                            rect.corner_radii.top_left / (command.bounding_box.height / 2.0),
                            10,
                            raylib::color::Color::new(rect.color.r as u8, rect.color.g as u8, rect.color.b as u8, rect.color.a as u8)
                        );
                    } else {
                        d.draw_rectangle(
                            command.bounding_box.x as i32,
                            command.bounding_box.y as i32,
                            command.bounding_box.width as i32,
                            command.bounding_box.height as i32,
                            raylib::color::Color::new(rect.color.r as u8, rect.color.g as u8, rect.color.b as u8, rect.color.a as u8),
                        );
                    }
                }
                RenderCommandConfig::Text(text) => {
                    let sanitized = text.text.replace('\0', "");
                    d.draw_text_ex(
                        &font,
                        &sanitized,
                        raylib::math::Vector2::new(command.bounding_box.x, command.bounding_box.y),
                        text.font_size as f32,
                        0.0,
                        raylib::color::Color::new(text.color.r as u8, text.color.g as u8, text.color.b as u8, text.color.a as u8),
                    );
                }
                RenderCommandConfig::ScissorStart() => {
                    unsafe {
                        raylib::ffi::BeginScissorMode(
                            command.bounding_box.x as i32,
                            command.bounding_box.y as i32,
                            command.bounding_box.width as i32,
                            command.bounding_box.height as i32,
                        );
                    }
                }
                RenderCommandConfig::ScissorEnd() => {
                    unsafe {
                        raylib::ffi::EndScissorMode();
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn render_node<'a, 'render>(
    clay: &mut clay_layout::ClayLayoutScope<'a, 'render, Texture2D, ()>, 
    node: &NodeInfo, 
    is_self: bool, 
    index: u32, 
    mouse_pressed: bool, 
    state: &Arc<Mutex<AppState>>,
    arena: &StringArena,
    font_scale: f32,
    rl: &mut RaylibHandle,
)
where
    'a: 'render,
{
    let node_id_label = node.id.as_deref().unwrap_or(&node.host_name);
    let node_id = clay.id_index(node_id_label, index);
    let is_online = node.online.unwrap_or(true);
    let is_exit_node = node.exit_node;
    let is_exit_node_option = node.exit_node_option;

    let mut bg_color = if is_online {
        Color::u_rgb(50, 50, 50)
    } else {
        Color::u_rgb(35, 35, 35)
    };

    let ip_str = node.tailscale_ips.get(0).cloned().unwrap_or_default();

    if clay.pointer_over(node_id) {
        bg_color = Color::u_rgb(70, 70, 70);
        if mouse_pressed && !ip_str.is_empty() {
            rl.set_clipboard_text(&ip_str).unwrap();
            let mut guard = state.lock().unwrap();
            guard.status_msg = Some(format!("Copied {} to clipboard", ip_str));
            guard.data_changed = true;
        }
    }

    if is_exit_node {
        bg_color = Color::u_rgb(0, 80, 0);
    }

    let mut node_decl = Declaration::new();
    node_decl.id(node_id)
        .layout()
            .width(grow!())
            .height(fixed!(80.0 * font_scale))
            .padding(Padding::all(12))
            .child_gap(12)
            .child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center))
        .end()
        .background_color(bg_color)
        .corner_radius().all(6.0).end();

    clay.with(&node_decl, |clay| {
        // Status Icon
        let status_icon = if is_online { ICON_CIRCLE } else { ICON_CIRCLE_OUTLINE };
        let status_color = if is_online { Color::u_rgb(0, 255, 0) } else { Color::u_rgb(150, 150, 150) };
        clay.text(status_icon, clay_layout::text::TextConfig::new().font_size((16.0 * font_scale) as u16).color(status_color).end());

        // OS Icon
        let os_icon = match node.os.as_deref() {
            Some("linux") => ICON_LINUX,
            Some("windows") => ICON_WINDOWS,
            Some("macos") | Some("darwin") | Some("ios") => ICON_APPLE,
            Some("android") => ICON_ANDROID,
            _ => ICON_LINUX,
        };
        clay.text(os_icon, clay_layout::text::TextConfig::new().font_size((20.0 * font_scale) as u16).color(Color::u_rgb(200, 200, 200)).end());

        let mut info_decl = Declaration::new();
        info_decl.layout().width(grow!()).child_gap(4).direction(LayoutDirection::TopToBottom).end();
        
        let hostname = arena.push(node.host_name.clone());
        let ip = if !ip_str.is_empty() { Some(arena.push(ip_str.clone())) } else { None };

        clay.with(&info_decl, |clay| {
            clay.text(hostname, clay_layout::text::TextConfig::new().font_size((18.0 * font_scale) as u16).color(Color::u_rgb(255, 255, 255)).end());
            if let Some(ip_val) = ip {
                clay.text(ip_val, clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(180, 180, 180)).end());
            }
        });

        // Use Exit Node Button / Stop Button
        if (is_exit_node_option && !is_self) || (is_self && is_exit_node) {
            let btn_id = clay.id_index(arena.push(format!("btn-{}", node_id_label)), index);
            let btn_text = if is_exit_node { format!("{} Stop", ICON_EXIT_NODE) } else { format!("{} Use Exit Node", ICON_EXIT_NODE) };
            let mut btn_bg = if is_exit_node { Color::u_rgb(150, 0, 0) } else { Color::u_rgb(0, 120, 0) };
            
            if clay.pointer_over(btn_id) {
                btn_bg = if is_exit_node { Color::u_rgb(200, 0, 0) } else { Color::u_rgb(0, 160, 0) };
                if mouse_pressed {
                    let state_clone = Arc::clone(state);
                    let ip_clone = ip_str.clone();
                    let disabling = is_exit_node;
                    tokio::spawn(async move {
                        {
                            let mut guard = state_clone.lock().unwrap();
                            guard.loading = true;
                        }
                        let res = if disabling {
                            tailscale::disable_exit_node().await
                        } else {
                            tailscale::set_exit_node(&ip_clone).await
                        };
                        match res {
                            Ok(_) => refresh_status(state_clone).await,
                            Err(e) => {
                                let mut guard = state_clone.lock().unwrap();
                                guard.error = Some(e.to_string());
                                guard.loading = false;
                            }
                        }
                    });
                }
            }

            let mut btn_decl = Declaration::new();
            btn_decl.id(btn_id)
                .layout()
                    .padding(Padding::horizontal(12))
                    .height(fixed!(40.0 * font_scale))
                    .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                    .child_gap(8)
                .end()
                .background_color(btn_bg)
                .corner_radius().all(4.0).end();
            
            let b_text = arena.push(btn_text);
            clay.with(&btn_decl, |clay| {
                clay.text(b_text, clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(255, 255, 255)).end());
            });
        } else if is_self {
             let me_text = arena.push(format!("{} (Me)", ICON_USER));
             clay.text(me_text, clay_layout::text::TextConfig::new().font_size((14.0 * font_scale) as u16).color(Color::u_rgb(150, 150, 255)).end());
        }
    });
}

async fn refresh_status(state: Arc<Mutex<AppState>>) {
    match tailscale::get_status().await {
        Ok(s) => {
            let mut guard = state.lock().unwrap();
            guard.status = Some(s);
            guard.loading = false;
            guard.error = None;
            guard.data_changed = true;
        }
        Err(e) => {
            let mut guard = state.lock().unwrap();
            guard.error = Some(e.to_string());
            guard.loading = false;
            guard.data_changed = true;
        }
    }
}
