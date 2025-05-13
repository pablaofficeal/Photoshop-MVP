use druid::widget::{Align, Button, Flex, Label, Painter, TextBox, WidgetExt};
use druid::{
    AppLauncher, Color, Data, Event, EventCtx, KeyEvent, Lens, LocalizedString, Point, Rect, RenderContext,
    Widget, WindowDesc, Code,
};
use druid::widget::Controller;
use druid::piet::ImageFormat;
use image::{Rgba, RgbaImage};
use image::imageops::replace;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

#[derive(Clone, Data, PartialEq)]
enum Tool {
    Brush,
    Eraser,
}

#[derive(Clone, Data, PartialEq)]
enum BrushShape {
    Square,
    Circle,
}

#[derive(Clone, Data, Lens)]
struct AppState {
    image: Arc<RwLock<RgbaImage>>,
    brush_color: Color,
    is_drawing: bool,
    brush_size: u32,
    current_tool: Tool,
    brush_shape: BrushShape,
    brush_size_input: String,
    color_r_input: String,
    color_g_input: String,
    color_b_input: String,
    background_color: Color,
}

struct CanvasController {
    last_paint: Instant,
}

impl CanvasController {
    fn new() -> Self {
        CanvasController {
            last_paint: Instant::now(),
        }
    }
}

impl<W: Widget<AppState>> Controller<AppState, W> for CanvasController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut AppState,
        env: &druid::Env,
    ) {
        let now = Instant::now();
        let should_paint = now.duration_since(self.last_paint) >= Duration::from_millis(16); // ~60 FPS

        match event {
            Event::MouseDown(mouse_event) => {
                data.is_drawing = true;
                draw_on_canvas(data, mouse_event.pos, ctx);
                if should_paint {
                    ctx.request_anim_frame();
                    self.last_paint = now;
                }
            }
            Event::MouseMove(mouse_event) if data.is_drawing => {
                draw_on_canvas(data, mouse_event.pos, ctx);
                if should_paint {
                    ctx.request_anim_frame();
                    self.last_paint = now;
                }
            }
            Event::MouseUp(_) => {
                data.is_drawing = false;
            }
            _ => {}
        }
        child.event(ctx, event, data, env);
    }
}

struct TextBoxController {
    is_brush_size: bool,
    is_color_r: bool,
    is_color_g: bool,
    is_color_b: bool,
}

impl TextBoxController {
    fn new(is_brush_size: bool, is_color_r: bool, is_color_g: bool, is_color_b: bool) -> Self {
        TextBoxController {
            is_brush_size,
            is_color_r,
            is_color_g,
            is_color_b,
        }
    }
}

impl<W: Widget<AppState>> Controller<AppState, W> for TextBoxController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut AppState,
        env: &druid::Env,
    ) {
        match event {
            Event::KeyDown(KeyEvent { code, .. }) if *code == Code::Enter => {
                if self.is_brush_size {
                    if let Ok(size) = data.brush_size_input.parse::<u32>() {
                        if size > 0 {
                            data.brush_size = size;
                        }
                    }
                }
                if self.is_color_r || self.is_color_g || self.is_color_b {
                    update_brush_color(data);
                }
            }
            _ => {}
        }
        child.event(ctx, event, data, env);
    }
}

fn main() {
    let window = WindowDesc::new(build_ui())
        .title(LocalizedString::new("Photoshop MVP"))
        .window_size((1200.0, 800.0));
    let mut initial_image = RgbaImage::new(800, 600);
    for pixel in initial_image.pixels_mut() {
        *pixel = Rgba([255, 255, 255, 255]);
    }
    let state = AppState {
        image: Arc::new(RwLock::new(initial_image)),
        brush_color: Color::BLACK,
        is_drawing: false,
        brush_size: 5,
        current_tool: Tool::Brush,
        brush_shape: BrushShape::Square,
        brush_size_input: "5".to_string(),
        color_r_input: "0".to_string(),
        color_g_input: "0".to_string(),
        color_b_input: "0".to_string(),
        background_color: Color::WHITE,
    };
    AppLauncher::with_window(window)
        .launch(state)
        .expect("Failed to launch application");
}

fn build_ui() -> impl Widget<AppState> {
    let canvas = Painter::new(|ctx, state: &AppState, _env| {
        let bounds = ctx.size().to_rect();
        ctx.fill(bounds, &state.background_color);

        let image = state.image.read().unwrap();
        let image_data = image.as_raw();
        let piet_image = ctx
            .make_image(
                image.width() as usize,
                image.height() as usize,
                image_data,
                ImageFormat::RgbaPremul,
            )
            .unwrap();
        ctx.draw_image(
            &piet_image,
            bounds,
            druid::piet::InterpolationMode::Bilinear,
        );
    })
    .fix_size(800.0, 600.0)
    .controller(CanvasController::new());

    let toolbar = Flex::column()
        .with_child(Label::new("Tools").with_text_size(18.0))
        .with_spacer(10.0)
        .with_child(
            Button::new("Brush")
                .on_click(|_ctx, state: &mut AppState, _env| {
                    state.current_tool = Tool::Brush;
                })
        )
        .with_child(
            Button::new("Eraser")
                .on_click(|_ctx, state: &mut AppState, _env| {
                    state.current_tool = Tool::Eraser;
                })
        )
        .with_spacer(10.0)
        .with_child(Label::new("Brush Shape").with_text_size(16.0))
        .with_child(
            Button::new("Square")
                .on_click(|_ctx, state: &mut AppState, _env| {
                    state.brush_shape = BrushShape::Square;
                })
        )
        .with_child(
            Button::new("Circle")
                .on_click(|_ctx, state: &mut AppState, _env| {
                    state.brush_shape = BrushShape::Circle;
                })
        )
        .with_spacer(10.0)
        .with_child(Label::new("Brush Size").with_text_size(16.0))
        .with_child(
            TextBox::new()
                .with_placeholder("Enter size (px)")
                .lens(AppState::brush_size_input)
                .controller(TextBoxController::new(true, false, false, false))
        )
        .with_spacer(10.0)
        .with_child(Label::new("Brush Color").with_text_size(16.0))
        .with_child(
            Flex::row()
                .with_child(
                    TextBox::new()
                        .with_placeholder("R (0-255)")
                        .lens(AppState::color_r_input)
                        .controller(TextBoxController::new(false, true, false, false))
                        .fix_width(60.0)
                )
                .with_child(
                    TextBox::new()
                        .with_placeholder("G (0-255)")
                        .lens(AppState::color_g_input)
                        .controller(TextBoxController::new(false, false, true, false))
                        .fix_width(60.0)
                )
                .with_child(
                    TextBox::new()
                        .with_placeholder("B (0-255)")
                        .lens(AppState::color_b_input)
                        .controller(TextBoxController::new(false, false, false, true))
                        .fix_width(60.0)
                )
        )
        .with_spacer(10.0)
        .with_child(Label::new("Color Palette").with_text_size(16.0))
        .with_child(
            Flex::column()
                .with_child(
                    Button::new("Red")
                        .on_click(|_ctx, state: &mut AppState, _env| {
                            state.brush_color = Color::rgb8(255, 0, 0);
                            state.color_r_input = "255".to_string();
                            state.color_g_input = "0".to_string();
                            state.color_b_input = "0".to_string();
                        })
                )
                .with_child(
                    Button::new("Green")
                        .on_click(|_ctx, state: &mut AppState, _env| {
                            state.brush_color = Color::rgb8(0, 255, 0);
                            state.color_r_input = "0".to_string();
                            state.color_g_input = "255".to_string();
                            state.color_b_input = "0".to_string();
                        })
                )
                .with_child(Button::new("Blue")
                        .on_click(|_ctx, state: &mut AppState, _env| {
                            state.brush_color = Color::rgb8(0, 0, 255);
                            state.color_r_input = "0".to_string();
                            state.color_g_input = "0".to_string();
                            state.color_b_input = "255".to_string();
                        })
                )
                .with_child(
                    Button::new("Cyan")
                        .on_click(|_ctx, state: &mut AppState, _env| {
                            state.brush_color = Color::rgb8(0, 255, 255);
                            state.color_r_input = "0".to_string();
                            state.color_g_input = "255".to_string();
                            state.color_b_input = "255".to_string();
                        })
                )
                .with_child(
                    Button::new("Brown")
                        .on_click(|_ctx, state: &mut AppState, _env| {
                            state.brush_color = Color::rgb8(139,69,19);
                            state.color_r_input = "139".to_string();
                            state.color_g_input = "69".to_string();
                            state.color_b_input = "19".to_string();
                        })
                )
                .with_child(
                    Button::new("Yellow")
                        .on_click(|_ctx, state: &mut AppState, _env| {
                            state.brush_color = Color::rgb8(255, 255, 0);
                            state.color_r_input = "255".to_string();
                            state.color_g_input = "255".to_string();
                            state.color_b_input = "0".to_string();
                        })
                )
                .with_child(
                    Button::new("test color")
                        .on_click(|_ctx, state: &mut AppState, _env| {
                            state.brush_color = Color::rgb8(196, 55, 140);
                            state.color_r_input = "196".to_string();
                            state.color_g_input = "55".to_string();
                            state.color_b_input = "140".to_string();
                        })
                )
                .with_child(
                    Button::new("test color1")
                        .on_click(|_ctx, state: &mut AppState, _env| {
                            state.brush_color = Color::rgb8(72,61,139);
                            state.color_r_input = "72".to_string();
                            state.color_g_input = "61".to_string();
                            state.color_b_input = "139".to_string();
                        })
                )
                .with_child(
                    Button::new("Black")
                        .on_click(|_ctx, state: &mut AppState, _env| {
                            state.brush_color = Color::rgb8(0, 0, 0);
                            state.color_r_input = "0".to_string();
                            state.color_g_input = "0".to_string();
                            state.color_b_input = "0".to_string();
                        })
                ))
        .with_spacer(10.0)
        .with_child(Label::new("Background Color").with_text_size(16.0))
        .with_child(
            Button::new("White")
                .on_click(|_ctx, state: &mut AppState, _env| {
                    set_background_color(state, Color::WHITE);
                })
        )
        .with_child(
            Button::new("Gray")
                .on_click(|_ctx, state: &mut AppState, _env| {
                    set_background_color(state, Color::rgb8(128, 128, 128));
                })
        )
        .with_child(
            Button::new("Transparent")
                .on_click(|_ctx, state: &mut AppState, _env| {
                    set_background_color(state, Color::rgba8(0, 0, 0, 0));
                })
        )
        .with_spacer(10.0)
        .with_child(
            Button::new("Save Image")
                .on_click(|_ctx, state: &mut AppState, _env| {
                    let image = state.image.read().unwrap();
                    image.save("output.png").expect("Failed to save image");
                })
        )
        .with_child(
            Button::new("Clear Canvas")
                .on_click(|_ctx, state: &mut AppState, _env| {
                    let mut image = state.image.write().unwrap();
                    for pixel in image.pixels_mut() {
                        *pixel = Rgba([255, 255, 255, 255]);
                    }
                })
        )
        .with_child(
            Button::new("EXIT")
                .on_click(|_ctx, _state: &mut AppState, _env| {
                    std::process::exit(0);
                })
        )
        .padding(10.0)
        .fix_width(200.0);

    Flex::row()
        .with_child(toolbar)
        .with_flex_spacer(1.0)
        .with_child(Align::centered(canvas))
        .with_flex_spacer(1.0)
        .padding(10.0)
}

fn set_background_color(state: &mut AppState, color: Color) {
    state.background_color = color.clone();
    let mut image = state.image.write().unwrap();
    let (r, g, b, a) = color.as_rgba8();
    for pixel in image.pixels_mut() {
        *pixel = Rgba([r, g, b, a]);
    }
}

fn update_brush_color(state: &mut AppState) {
    let r = state
        .color_r_input
        .parse::<u8>()
        .unwrap_or(0)
        .clamp(0, 255);
    let g = state
        .color_g_input
        .parse::<u8>()
        .unwrap_or(0)
        .clamp(0, 255);
    let b = state
        .color_b_input
        .parse::<u8>()
        .unwrap_or(0)
        .clamp(0, 255);
    state.brush_color = Color::rgb8(r, g, b);
}

fn draw_on_canvas(state: &mut AppState, pos: Point, ctx: &mut EventCtx) {
    let mut image = state.image.write().unwrap();
    let x_center = (pos.x * image.width() as f64 / 800.0) as i32;
    let y_center = (pos.y * image.height() as f64 / 600.0) as i32;
    let radius = state.brush_size as i32;

    let color = match state.current_tool {
        Tool::Brush => {
            let (r, g, b, a) = state.brush_color.as_rgba8();
            Rgba([r, g, b, a])
        }
        Tool::Eraser => {
            let (r, g, b, a) = state.background_color.as_rgba8();
            Rgba([r, g, b, a])
        }
    };

    match state.brush_shape {
        BrushShape::Square => {
            let x_min = (x_center - radius).max(0) as u32;
            let x_max = (x_center + radius + 1).min(image.width() as i32) as u32;
            let y_min = (y_center - radius).max(0) as u32;
            let y_max = (y_center + radius + 1).min(image.height() as i32) as u32;

            let brush = RgbaImage::from_pixel(
                (x_max - x_min) as u32,
                (y_max - y_min) as u32,
                color,
            );
            replace(&mut *image, &brush, x_min as i64, y_min as i64);
        }
        BrushShape::Circle => {
            for x in (x_center - radius).max(0)..=(x_center + radius).min(image.width() as i32 - 1) {
                for y in (y_center - radius).max(0)..=(y_center + radius).min(image.height() as i32 - 1) {
                    let dx = x - x_center;
                    let dy = y - y_center;
                    if dx * dx + dy * dy <= radius * radius {
                        image.put_pixel(x as u32, y as u32, color);
                    }
                }
            }
        }
    }

    let dirty_rect = Rect::from_origin_size(
        Point::new(
            (x_center - radius) as f64 * 800.0 / image.width() as f64,
            (y_center - radius) as f64 * 600.0 / image.height() as f64,
        ),
        (
            (radius * 2) as f64 * 800.0 / image.width() as f64,
            (radius * 2) as f64 * 600.0 / image.height() as f64,
        ),
    );
    ctx.request_paint_rect(dirty_rect);
}
