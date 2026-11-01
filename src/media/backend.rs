use super::{audio::AudioEngine, WindowConfig};
use crate::runtime::values::Value;
use sdl2::{
    event::Event,
    keyboard::Keycode,
    mouse::MouseButton,
    pixels::{Color, PixelFormatEnum},
    rect::{Point, Rect},
    render::{BlendMode, Canvas},
    video::FullscreenType,
};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

#[derive(Default)]
pub struct Media {
    pub config: WindowConfig,
    audio: Option<AudioEngine>,
    canvas: Option<Canvas<sdl2::video::Window>>,
    events: Option<sdl2::EventPump>,
    sdl: Option<sdl2::Sdl>,
    images: HashMap<String, image::RgbaImage>,
    texts: HashMap<(String, u32), image::RgbaImage>,
    font: Option<fontdue::Font>,
    keys: HashSet<Keycode>,
    tick_pressed: HashSet<Keycode>,
    frame_pressed: HashSet<Keycode>,
    mouse_down: HashSet<MouseButton>,
    tick_mouse: HashSet<MouseButton>,
    frame_mouse: HashSet<MouseButton>,
    mouse: (i32, i32),
    tick_phase: bool,
    windowed_size: Option<(u32, u32)>,
}
impl Media {
    fn sdl(&mut self) -> Result<&sdl2::Sdl, String> {
        if self.sdl.is_none() {
            self.sdl = Some(sdl2::init()?);
        }
        Ok(self.sdl.as_ref().unwrap())
    }
    pub fn backend(&self) -> String {
        self.canvas
            .as_ref()
            .map(|c| c.info().name.to_string())
            .unwrap_or("not initialized".into())
    }
    pub fn open_window(&mut self) -> Result<(), String> {
        if self.canvas.is_some() {
            return Ok(());
        }
        let video = self.sdl()?.video()?;
        let index = sdl2::render::drivers()
            .position(|d| d.name == "opengl")
            .ok_or("SDL has no OpenGL renderer on this system")?;
        let window = video
            .window(&self.config.title, self.config.width, self.config.height)
            .opengl()
            .resizable()
            .position_centered()
            .build()
            .map_err(|e| format!("Cannot open OpenGL window: {e}"))?;
        let mut builder = window.into_canvas().index(index as u32);
        if self.config.vsync {
            builder = builder.present_vsync();
        }
        let mut canvas = builder
            .build()
            .map_err(|e| format!("Cannot create OpenGL renderer: {e}"))?;
        canvas.set_blend_mode(BlendMode::Blend);
        self.canvas = Some(canvas);
        self.events = Some(self.sdl()?.event_pump()?);
        self.configure(self.config.clone())?;
        Ok(())
    }
    pub fn configure(&mut self, mut c: WindowConfig) -> Result<(), String> {
        if !c.fullscreen && self.config.fullscreen {
            if let Some((w, h)) = self.windowed_size {
                c.width = w;
                c.height = h;
            }
        }
        // SDL's offscreen EGL pbuffer retains its original dimensions on resize.
        // Recreate only that driver's window to allocate a correctly sized buffer.
        // Native desktop windows keep their existing context and window identity.
        let recreate = self.canvas.as_ref().is_some_and(|canvas| {
            !c.fullscreen
                && canvas.window().subsystem().current_video_driver() == "offscreen"
                && canvas.window().size() != (c.width, c.height)
        });
        if recreate {
            self.events = None;
            self.canvas = None;
            self.config = c;
            self.keys.clear();
            self.mouse_down.clear();
            self.tick_pressed.clear();
            self.frame_pressed.clear();
            self.tick_mouse.clear();
            self.frame_mouse.clear();
            return self.open_window();
        }
        if let Some(canvas) = &mut self.canvas {
            if self.config.title != c.title {
                canvas
                    .window_mut()
                    .set_title(&c.title)
                    .map_err(|e| e.to_string())?;
            }
            if c.fullscreen && !self.config.fullscreen {
                self.windowed_size = Some(canvas.window().size());
            }
            if !c.fullscreen && self.config.fullscreen {
                if let Some((w, h)) = self.windowed_size {
                    c.width = w;
                    c.height = h;
                }
            }
            let wanted = if c.fullscreen {
                if c.exclusive {
                    FullscreenType::True
                } else {
                    FullscreenType::Desktop
                }
            } else {
                FullscreenType::Off
            };
            if canvas.window().fullscreen_state() != wanted {
                canvas.window_mut().set_fullscreen(wanted)?;
            }
            if !c.fullscreen && canvas.window().size() != (c.width, c.height) {
                canvas
                    .window_mut()
                    .set_size(c.width, c.height)
                    .map_err(|e| e.to_string())?;
            }
            if c.vsync != self.config.vsync {
                // SAFETY: the renderer belongs to this live canvas and is used only on this thread.
                let result =
                    unsafe { sdl2::sys::SDL_RenderSetVSync(canvas.raw(), i32::from(c.vsync)) };
                if result != 0 {
                    return Err(format!("Cannot change VSync: {}", sdl2::get_error()));
                }
            }
        }
        self.config = c;
        Ok(())
    }
    pub fn poll_events(&mut self) -> Result<bool, String> {
        let Some(events) = &mut self.events else {
            return Err("Window is not initialized".into());
        };
        let mut quit = false;
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } => quit = true,
                Event::KeyDown {
                    keycode: Some(key),
                    repeat: false,
                    ..
                } => {
                    self.keys.insert(key);
                    self.tick_pressed.insert(key);
                    self.frame_pressed.insert(key);
                }
                Event::KeyUp {
                    keycode: Some(key), ..
                } => {
                    self.keys.remove(&key);
                }
                Event::MouseMotion { x, y, .. } => self.mouse = (x, y),
                Event::MouseButtonDown {
                    mouse_btn, x, y, ..
                } => {
                    self.mouse = (x, y);
                    self.mouse_down.insert(mouse_btn);
                    self.tick_mouse.insert(mouse_btn);
                    self.frame_mouse.insert(mouse_btn);
                }
                Event::MouseButtonUp { mouse_btn, .. } => {
                    self.mouse_down.remove(&mouse_btn);
                }
                Event::Window {
                    win_event: sdl2::event::WindowEvent::FocusLost,
                    ..
                } => {
                    self.keys.clear();
                    self.mouse_down.clear();
                    self.tick_pressed.clear();
                    self.frame_pressed.clear();
                    self.tick_mouse.clear();
                    self.frame_mouse.clear();
                }
                Event::Window {
                    win_event: sdl2::event::WindowEvent::SizeChanged(w, h),
                    ..
                } if w > 0 && h > 0 => {
                    self.config.width = w as u32;
                    self.config.height = h as u32;
                    if !self.config.fullscreen {
                        self.windowed_size = Some((w as u32, h as u32));
                    }
                }
                _ => {}
            }
        }
        Ok(quit)
    }
    pub fn mouse(&self) -> (i32, i32) {
        self.mouse
    }
    pub fn tick_phase(&mut self) {
        self.tick_phase = true;
    }
    pub fn frame_phase(&mut self) {
        self.tick_phase = false;
    }
    pub fn end_tick(&mut self) {
        self.tick_pressed.clear();
        self.tick_mouse.clear();
    }
    pub fn end_frame(&mut self) {
        self.frame_pressed.clear();
        self.frame_mouse.clear();
    }
    pub fn input(&self, action: &str, key: &str) -> Result<bool, String> {
        if self.canvas.is_none() {
            return Err("Keyboard and mouse input require an initialized 2d window".into());
        }
        match action {
            "key_down" | "key_pressed" => {
                let key = Keycode::from_name(key)
                    .ok_or_else(|| format!("Unknown keyboard key '{key}'"))?;
                Ok(if action == "key_down" {
                    self.keys.contains(&key)
                } else if self.tick_phase {
                    self.tick_pressed.contains(&key)
                } else {
                    self.frame_pressed.contains(&key)
                })
            }
            "mouse_down" | "mouse_pressed" => {
                let key = match key {
                    "left" => MouseButton::Left,
                    "middle" => MouseButton::Middle,
                    "right" => MouseButton::Right,
                    _ => return Err("Mouse button must be left, middle, or right".into()),
                };
                Ok(if action == "mouse_down" {
                    self.mouse_down.contains(&key)
                } else if self.tick_phase {
                    self.tick_mouse.contains(&key)
                } else {
                    self.frame_mouse.contains(&key)
                })
            }
            _ => Err(format!("Unknown input function '{action}'")),
        }
    }
    pub fn present(&mut self) {
        if let Some(c) = &mut self.canvas {
            c.present();
        }
    }
    pub fn play_audio(&mut self, path: &Path, volume: f32, looping: bool) -> Result<u64, String> {
        if self.audio.is_none() {
            self.audio = Some(AudioEngine::new(self.sdl()?)?);
        }
        self.audio.as_mut().unwrap().play(path, volume, looping)
    }
    pub fn audio_control(
        &mut self,
        action: &str,
        id: i64,
        volume: Option<f32>,
    ) -> Result<bool, String> {
        match &mut self.audio {
            Some(a) => a.control(action, id, volume),
            None => Ok(false),
        }
    }
    pub fn draw(&mut self, n: &str, args: &[Value], file: &str) -> Result<(), String> {
        if self.canvas.is_none() {
            return Err("Draw inside 'every frame { ... } loop', after the window opens".into());
        }
        let a = DrawArgs(args);
        match n {
            "clear" => {
                a.count(1, 1)?;
                let color = a.color(0)?;
                let c = self.canvas.as_mut().unwrap();
                c.set_draw_color(color);
                c.clear();
            }
            "rect" | "outline" => {
                a.count(4, 5)?;
                let rect = Rect::new(a.coord(0)?, a.coord(1)?, a.size(2)?, a.size(3)?);
                let color = a.color(4)?;
                let c = self.canvas.as_mut().unwrap();
                c.set_draw_color(color);
                if n == "rect" {
                    c.fill_rect(rect)?
                } else {
                    c.draw_rect(rect)?
                }
            }
            "line" => {
                a.count(4, 5)?;
                let (x1, y1, x2, y2) = (a.coord(0)?, a.coord(1)?, a.coord(2)?, a.coord(3)?);
                let color = a.color(4)?;
                let c = self.canvas.as_mut().unwrap();
                c.set_draw_color(color);
                c.draw_line(Point::new(x1, y1), Point::new(x2, y2))?;
            }
            "circle" => {
                a.count(3, 4)?;
                let (x, y) = (a.coord(0)?, a.coord(1)?);
                let radius = a.size(2)? as i32;
                let color = a.color(3)?;
                let c = self.canvas.as_mut().unwrap();
                c.set_draw_color(color);
                for dy in -radius..=radius {
                    let dx = ((radius as f64).powi(2) - (dy as f64).powi(2)).sqrt() as i32;
                    c.draw_line(Point::new(x - dx, y + dy), Point::new(x + dx, y + dy))?;
                }
            }
            "image" => {
                a.count(3, 5)?;
                if args.len() == 4 {
                    return Err(
                        "renderer.image expects (path, x, y) or (path, x, y, width, height)".into(),
                    );
                }
                let path = crate::loader::resolve_import(file, a.text(0)?);
                let path = path
                    .canonicalize()
                    .map_err(|e| format!("Cannot open image: {e}"))?;
                let key = path.to_string_lossy().into_owned();
                if !self.images.contains_key(&key) {
                    let mut reader = image::io::Reader::open(&path)
                        .map_err(|e| e.to_string())?
                        .with_guessed_format()
                        .map_err(|e| e.to_string())?;
                    let mut limits = image::io::Limits::default();
                    limits.max_image_width = Some(8192);
                    limits.max_image_height = Some(8192);
                    limits.max_alloc = Some(128 * 1024 * 1024);
                    reader.limits(limits);
                    let image = reader
                        .decode()
                        .map_err(|e| format!("Cannot decode image: {e}"))?
                        .into_rgba8();
                    self.images.insert(key.clone(), image);
                }
                let image = &self.images[&key];
                let w = if args.len() == 5 {
                    a.size(3)?
                } else {
                    image.width()
                };
                let h = if args.len() == 5 {
                    a.size(4)?
                } else {
                    image.height()
                };
                blit(
                    self.canvas.as_mut().unwrap(),
                    image,
                    Rect::new(a.coord(1)?, a.coord(2)?, w, h),
                    Color::WHITE,
                )?;
            }
            "text" => {
                a.count(3, 5)?;
                let text = a.text(0)?;
                let size = if args.len() > 3 {
                    a.number(3)? as f32
                } else {
                    24.0
                };
                if !(1.0..=256.0).contains(&size) {
                    return Err("Font size must be between 1 and 256".into());
                }
                if text.len() > 16384 {
                    return Err("Text draw exceeds 16384 bytes".into());
                }
                if text.is_empty() {
                    return Ok(());
                }
                let key = (text.to_owned(), size.to_bits());
                if !self.texts.contains_key(&key) {
                    if self.font.is_none() {
                        self.font = Some(
                            fontdue::Font::from_bytes(
                                include_bytes!("../../assets/DejaVuSans.ttf") as &[u8],
                                fontdue::FontSettings::default(),
                            )
                            .map_err(|e| e.to_string())?,
                        );
                    }
                    let font = self.font.as_ref().unwrap();
                    let rendered = rasterize_text(font, text, size)?;
                    if self.texts.len() >= 128 {
                        self.texts.clear();
                    }
                    self.texts.insert(key.clone(), rendered);
                }
                let image = &self.texts[&key];
                let rect = Rect::new(a.coord(1)?, a.coord(2)?, image.width(), image.height());
                let color = a.color(4)?;
                blit(self.canvas.as_mut().unwrap(), image, rect, color)?;
            }
            "save" => {
                a.count(1, 1)?;
                let path = crate::loader::resolve_import(file, a.text(0)?);
                let c = self.canvas.as_mut().unwrap();
                let (w, h) = c.output_size()?;
                let pixels = c.read_pixels(None, PixelFormatEnum::RGBA32)?;
                image::save_buffer(path, &pixels, w, h, image::ColorType::Rgba8)
                    .map_err(|e| e.to_string())?;
            }
            _ => return Err(format!("Unknown drawing function '{n}'")),
        }
        Ok(())
    }
}
fn rasterize_text(font: &fontdue::Font, text: &str, size: f32) -> Result<image::RgbaImage, String> {
    use fontdue::layout::{CoordinateSystem, Layout, LayoutSettings, TextStyle};
    let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
    layout.reset(&LayoutSettings::default());
    layout.append(&[font], &TextStyle::new(text, size, 0));
    let glyphs = layout.glyphs();
    let w = glyphs
        .iter()
        .map(|g| (g.x + g.width as f32).ceil().max(0.0) as u32)
        .max()
        .unwrap_or(1)
        .max(1);
    let h = layout.height().ceil().max(size) as u32;
    if w > 16384 || h > 4096 {
        return Err("Rendered text exceeds maximum dimensions".into());
    }
    let mut result = image::RgbaImage::new(w, h);
    for glyph in glyphs {
        let (_, bitmap) = font.rasterize_config(glyph.key);
        for y in 0..glyph.height {
            for x in 0..glyph.width {
                let dx = glyph.x as i32 + x as i32;
                let dy = glyph.y as i32 + y as i32;
                if dx >= 0 && dy >= 0 && (dx as u32) < w && (dy as u32) < h {
                    result.put_pixel(
                        dx as u32,
                        dy as u32,
                        image::Rgba([255, 255, 255, bitmap[y * glyph.width + x]]),
                    );
                }
            }
        }
    }
    Ok(result)
}
fn blit(
    canvas: &mut Canvas<sdl2::video::Window>,
    image: &image::RgbaImage,
    rect: Rect,
    color: Color,
) -> Result<(), String> {
    let mut surface =
        sdl2::surface::Surface::new(image.width(), image.height(), PixelFormatEnum::RGBA32)?;
    let pitch = surface.pitch() as usize;
    surface.with_lock_mut(|bytes| {
        for (y, row) in image
            .as_raw()
            .chunks_exact(image.width() as usize * 4)
            .enumerate()
        {
            bytes[y * pitch..y * pitch + row.len()].copy_from_slice(row);
        }
    });
    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_from_surface(&surface)
        .map_err(|e| e.to_string())?;
    texture.set_blend_mode(BlendMode::Blend);
    texture.set_color_mod(color.r, color.g, color.b);
    texture.set_alpha_mod(color.a);
    canvas.copy(&texture, None, rect)
}
struct DrawArgs<'a>(&'a [Value]);
impl<'a> DrawArgs<'a> {
    fn count(&self, min: usize, max: usize) -> Result<(), String> {
        if self.0.len() < min || self.0.len() > max {
            Err(format!(
                "Drawing expects {min}..{max} arguments, got {}",
                self.0.len()
            ))
        } else {
            Ok(())
        }
    }
    fn text(&self, n: usize) -> Result<&'a str, String> {
        if let Some(Value::Text(s)) = self.0.get(n) {
            Ok(s)
        } else {
            Err(format!("Argument {} must be text", n + 1))
        }
    }
    fn number(&self, n: usize) -> Result<f64, String> {
        self.0
            .get(n)
            .and_then(Value::number)
            .ok_or_else(|| format!("Argument {} must be a number", n + 1))
    }
    fn coord(&self, n: usize) -> Result<i32, String> {
        let x = self.number(n)?;
        if !(-1_000_000.0..=1_000_000.0).contains(&x) {
            Err("Drawing coordinate is outside -1000000..1000000".into())
        } else {
            Ok(x as i32)
        }
    }
    fn size(&self, n: usize) -> Result<u32, String> {
        let x = self.number(n)?;
        if !(1.0..=16384.0).contains(&x) {
            Err("Drawing dimensions must be between 1 and 16384".into())
        } else {
            Ok(x as u32)
        }
    }
    fn color(&self, n: usize) -> Result<Color, String> {
        let Some(v) = self.0.get(n) else {
            return Ok(Color::WHITE);
        };
        if let Value::Text(s) = v {
            let s = match s.as_str() {
                "black" => "#000000",
                "white" => "#ffffff",
                "red" => "#ff0000",
                "green" => "#00ff00",
                "blue" => "#0000ff",
                "indium" => "#7163ff",
                s => s,
            };
            if s.starts_with('#')
                && (s.len() == 7 || s.len() == 9)
                && s[1..].bytes().all(|c| c.is_ascii_hexdigit())
            {
                let r = u8::from_str_radix(&s[1..3], 16).unwrap();
                let g = u8::from_str_radix(&s[3..5], 16).unwrap();
                let b = u8::from_str_radix(&s[5..7], 16).unwrap();
                let alpha = if s.len() == 9 {
                    u8::from_str_radix(&s[7..9], 16).unwrap()
                } else {
                    255
                };
                return Ok(Color::RGBA(r, g, b, alpha));
            }
        }
        if let Value::List(xs) = v {
            let xs = xs.borrow();
            if xs.len() == 3 || xs.len() == 4 {
                let mut c = [255u8; 4];
                for (i, v) in xs.iter().enumerate() {
                    if let Value::Integer(n) = v {
                        if (0..=255).contains(n) {
                            c[i] = *n as u8;
                            continue;
                        }
                    }
                    return Err("Color channels must be integers between 0 and 255".into());
                }
                return Ok(Color::RGBA(c[0], c[1], c[2], c[3]));
            }
        }
        Err("Color must be #RRGGBB, #RRGGBBAA, a named color, or [red, green, blue, alpha]".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires a working OpenGL display; SDL_VIDEODRIVER=offscreen works with Mesa"]
    fn opengl_pixels_input_resize_and_fullscreen() {
        let mut media = Media::default();
        media.config.width = 320;
        media.config.height = 240;
        media.config.vsync = false;
        media.open_window().unwrap();
        assert_eq!(media.backend(), "opengl");
        media.poll_events().unwrap();
        let event = media.sdl.as_ref().unwrap().event().unwrap();
        event
            .push_event(Event::KeyDown {
                timestamp: 0,
                window_id: 0,
                keycode: Some(Keycode::Space),
                scancode: Some(sdl2::keyboard::Scancode::Space),
                keymod: sdl2::keyboard::Mod::NOMOD,
                repeat: false,
            })
            .unwrap();
        media.poll_events().unwrap();
        media.tick_phase();
        assert!(media.input("key_down", "space").unwrap());
        assert!(media.input("key_pressed", "space").unwrap());
        media.end_tick();
        assert!(!media.input("key_pressed", "space").unwrap());
        media.frame_phase();
        assert!(media.input("key_pressed", "space").unwrap());
        media.end_frame();
        assert!(!media.input("key_pressed", "space").unwrap());
        event
            .push_event(Event::KeyUp {
                timestamp: 0,
                window_id: 0,
                keycode: Some(Keycode::Space),
                scancode: Some(sdl2::keyboard::Scancode::Space),
                keymod: sdl2::keyboard::Mod::NOMOD,
                repeat: false,
            })
            .unwrap();
        media.poll_events().unwrap();
        assert!(!media.input("key_down", "space").unwrap());
        let mut c = media.config.clone();
        c.width = 400;
        c.height = 300;
        media.configure(c).unwrap();
        assert_eq!(
            media.canvas.as_ref().unwrap().output_size().unwrap(),
            (400, 300)
        );
        let mut c = media.config.clone();
        c.fullscreen = true;
        match media.configure(c) {
            Ok(()) => {
                assert_ne!(
                    media.canvas.as_ref().unwrap().window().fullscreen_state(),
                    FullscreenType::Off
                );
                let mut c = media.config.clone();
                c.fullscreen = false;
                media.configure(c).unwrap();
                assert_eq!(
                    media.canvas.as_ref().unwrap().window().fullscreen_state(),
                    FullscreenType::Off
                );
            }
            Err(e) => eprintln!("Fullscreen unsupported on this test display: {e}"),
        }
        media.poll_events().unwrap();
        media.present();
        media
            .draw("clear", &[Value::Text("black".into())], "test.in")
            .unwrap();
        media
            .draw(
                "rect",
                &[
                    Value::Integer(10),
                    Value::Integer(10),
                    Value::Integer(20),
                    Value::Integer(20),
                    Value::Text("red".into()),
                ],
                "test.in",
            )
            .unwrap();
        let pixels = media
            .canvas
            .as_ref()
            .unwrap()
            .read_pixels(Some(Rect::new(15, 15, 1, 1)), PixelFormatEnum::RGBA32)
            .unwrap();
        assert_eq!(&pixels[..3], &[255, 0, 0]);
        assert!(media.input("key_down", "nonexistent-key-49").is_err());
    }
}
