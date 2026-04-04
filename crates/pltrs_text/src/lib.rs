use anyhow::{Context, anyhow};
use pltrs_core::TextBatch;
use std::{fs, path::PathBuf};
use wgpu_text::{
    BrushBuilder, TextBrush,
    glyph_brush::{Section as TextSection, Text, ab_glyph::FontArc},
};

pub struct TextRenderer {
    brush: TextBrush<FontArc>,
}

impl TextRenderer {
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> anyhow::Result<Self> {
        let font = load_font()?;
        let brush = BrushBuilder::using_font(font).build(device, width, height, format);
        Ok(Self { brush })
    }

    pub fn queue(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        batches: &[TextBatch],
    ) {
        if batches.is_empty() {
            return;
        }

        let sections: Vec<TextSection> = batches
            .iter()
            .map(|batch| {
                let x_px = batch.position[0] * width as f32;
                let y_px = (1.0 - batch.position[1]) * height as f32;

                TextSection::default()
                    .with_screen_position((x_px, y_px))
                    .add_text(
                        Text::new(&batch.content)
                            .with_scale(batch.size)
                            .with_color([
                                batch.color.r,
                                batch.color.g,
                                batch.color.b,
                                batch.color.a,
                            ]),
                    )
            })
            .collect();

        if let Err(err) = self.brush.queue(device, queue, sections) {
            eprintln!("Text queue error: {:?}", err);
        }
    }

    pub fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        has_text: bool,
    ) {
        if !has_text {
            return;
        }

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Text Draw Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        self.brush.draw(&mut rpass);
    }

    pub fn resize(&self, width: u32, height: u32, queue: &wgpu::Queue) {
        self.brush.resize_view(width as f32, height as f32, queue);
    }
}

fn load_font() -> anyhow::Result<FontArc> {
    if let Some(font_path) = std::env::var_os("PLTRS_FONT_PATH") {
        let path = PathBuf::from(font_path);
        let font_bytes =
            fs::read(&path).with_context(|| format!("failed to read font from {:?}", path))?;
        return FontArc::try_from_vec(font_bytes)
            .map_err(|_| anyhow!("failed to parse font at {:?}", path));
    }

    FontArc::try_from_slice(include_bytes!("../assets/NotoSans[wght].ttf"))
        .map_err(|_| anyhow!("failed to parse bundled default font"))
}
