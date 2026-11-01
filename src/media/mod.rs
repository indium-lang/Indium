#[derive(Clone, Debug)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub vsync: bool,
    pub fullscreen: bool,
    pub exclusive: bool,
}
impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Indium".into(),
            width: 800,
            height: 600,
            fps: 60,
            vsync: true,
            fullscreen: false,
            exclusive: false,
        }
    }
}
#[cfg(feature = "media")]
mod audio;
#[cfg(feature = "media")]
mod backend;
#[cfg(feature = "media")]
pub use backend::Media;
#[cfg(not(feature = "media"))]
mod headless {
    use super::WindowConfig;
    use crate::runtime::values::Value;
    #[derive(Default)]
    pub struct Media {
        pub config: WindowConfig,
    }
    const UNAVAILABLE:&str="This build has no media backend. Rebuild with default features for OpenGL graphics and OGG/WAV audio.";
    impl Media {
        pub fn open_window(&mut self) -> Result<(), String> {
            Err(UNAVAILABLE.into())
        }
        pub fn configure(&mut self, c: WindowConfig) -> Result<(), String> {
            self.config = c;
            Ok(())
        }
        pub fn backend(&self) -> String {
            "unavailable".into()
        }
        pub fn mouse(&self) -> (i32, i32) {
            (0, 0)
        }
        pub fn poll_events(&mut self) -> Result<bool, String> {
            Err(UNAVAILABLE.into())
        }
        pub fn tick_phase(&mut self) {}
        pub fn frame_phase(&mut self) {}
        pub fn end_tick(&mut self) {}
        pub fn end_frame(&mut self) {}
        pub fn present(&mut self) {}
        pub fn input(&self, _: &str, _: &str) -> Result<bool, String> {
            Err(UNAVAILABLE.into())
        }
        pub fn draw(&mut self, _: &str, _: &[Value], _: &str) -> Result<(), String> {
            Err(UNAVAILABLE.into())
        }
        pub fn play_audio(&mut self, _: &std::path::Path, _: f32, _: bool) -> Result<u64, String> {
            Err(UNAVAILABLE.into())
        }
        pub fn audio_control(&mut self, _: &str, _: i64, _: Option<f32>) -> Result<bool, String> {
            Err(UNAVAILABLE.into())
        }
    }
}
#[cfg(not(feature = "media"))]
pub use headless::Media;
