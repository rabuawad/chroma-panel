use crate::colors::channels;
use gpui::RenderImage;
use image::{Frame, RgbaImage};
use std::sync::{Arc, LazyLock};

static SCENE: LazyLock<RgbaImage> = LazyLock::new(|| {
    image::load_from_memory(include_bytes!("../assets/mock-desktop.png"))
        .expect("embedded mock desktop PNG")
        .into_rgba8()
});

/// Same color multiplier as the GNOME fragment shader at full intensity.
pub fn tint_multiplier(channel: u8) -> f32 {
    0.35 + 0.65 * f32::from(channel) / 255.
}

#[derive(Default)]
pub struct DesktopPreview {
    cached: Option<((u32, u32), Arc<RenderImage>)>,
}

impl DesktopPreview {
    pub fn image(&mut self, color: u32, intensity: u32) -> Arc<RenderImage> {
        let key = (if intensity == 0 { 0 } else { color }, intensity);
        if let Some((previous, image)) = &self.cached
            && *previous == key
        {
            return image.clone();
        }
        let tables = channels(color).map(|channel| {
            let gain = 1. + (tint_multiplier(channel) - 1.) * intensity as f32 / 100.;
            std::array::from_fn::<u8, 256, _>(|n| (n as f32 * gain).round() as u8)
        });
        let mut pixels = SCENE.clone();
        for pixel in pixels.as_mut().chunks_exact_mut(4) {
            let r = tables[0][pixel[0] as usize];
            pixel[1] = tables[1][pixel[1] as usize];
            pixel[0] = tables[2][pixel[2] as usize];
            pixel[2] = r;
        }
        let image = Arc::new(RenderImage::new([Frame::new(pixels)]));
        self.cached = Some((key, image.clone()));
        image
    }
}
