use crate::*;

use rusttype::gpu_cache::Cache;
use rusttype::{point, Font, PositionedGlyph, Rect, Scale};

use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, CpuBufferPool, TypedBufferAccess};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CopyBufferToImageInfo, PrimaryAutoCommandBuffer, RenderPassBeginInfo,
    SubpassContents,
};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::{Device, Queue};
use vulkano::format::{ClearValue, Format};
use vulkano::image::view::ImageView;
use vulkano::image::ImageAccess;
use vulkano::image::{
    ImageCreateFlags, ImageDimensions, ImageLayout, ImageUsage, ImmutableImage, SwapchainImage,
};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::{Framebuffer, Subpass};
use vulkano::sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode};
use vulkano::swapchain::{self, AcquireError, Swapchain};

use bytemuck::{Pod, Zeroable};

use std::iter;
use std::sync::Arc;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Zeroable, Pod)]
struct Vertex {
    position: [f32; 2],
    tex_position: [f32; 2],
    color: [f32; 4],
}
vulkano::impl_vertex!(Vertex, position, tex_position, color);

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/shaders/vertex.glsl",
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/shaders/fragment.glsl",
    }
}

struct TextData {
    glyphs: Vec<PositionedGlyph<'static>>,
    color: [f32; 4],
}

pub struct DrawText {
    device: Arc<Device>,
    queue: Arc<Queue>,
    font: Font<'static>,
    cache: Cache<'static>,
    cache_pixel_buffer: Vec<u8>,
    pipeline: Arc<GraphicsPipeline>,
    framebuffers: Vec<Arc<Framebuffer>>,
    texts: Vec<TextData>,
}

const CACHE_WIDTH: usize = 1000;
const CACHE_HEIGHT: usize = 1000;

impl DrawText {
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        swapchain: Arc<Swapchain>,
        images: &[Arc<SwapchainImage>],
    ) -> DrawText {
        let font_data = include_bytes!("../font/DejaVuSans.ttf");
        let font = Font::from_bytes(font_data as &[u8]).unwrap();

        let vs = vs::load(device.clone()).unwrap();
        let fs = fs::load(device.clone()).unwrap();

        let cache = Cache::builder()
            .dimensions(CACHE_WIDTH as u32, CACHE_HEIGHT as u32)
            .build();
        let cache_pixel_buffer = vec![0; CACHE_WIDTH * CACHE_HEIGHT];

        let render_pass = vulkano::single_pass_renderpass!(device.clone(),
            attachments: {
                color: {
                    load: Load,
                    store: Store,
                    format: swapchain.image_format(),
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        )
        .unwrap();

        let framebuffers = images
            .iter()
            .map(|image| {
                let view = ImageView::new_default(image.clone()).unwrap();
                Framebuffer::new(
                    render_pass.clone(),
                    FramebufferCreateInfo {
                        attachments: vec![view],
                        ..Default::default()
                    },
                )
                .unwrap()
            })
            .collect::<Vec<_>>();

        let pipeline = GraphicsPipeline::start()
            .vertex_input_single_buffer::<Vertex>()
            .vertex_shader(vs.entry_point("main").unwrap(), ())
            .triangle_list()
            .viewports(iter::once(Viewport {
                origin: [0.0, 0.0],
                depth_range: 0.0..1.0,
                dimensions: [
                    images[0].dimensions().width_height()[0] as f32,
                    images[0].dimensions().width_height()[1] as f32,
                ],
            }))
            .fragment_shader(fs.entry_point("main").unwrap(), ())
            .blend_alpha_blending()
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap();

        DrawText {
            device,
            queue,
            font,
            cache,
            cache_pixel_buffer,
            pipeline,
            framebuffers,
            texts: vec![],
        }
    }

    pub fn queue_text(&mut self, x: f32, y: f32, size: f32, color: [f32; 4], text: &str) {
        let glyphs: Vec<PositionedGlyph> = self
            .font
            .layout(text, Scale::uniform(size), point(x, y))
            .map(|x| x.standalone())
            .collect();
        for glyph in &glyphs {
            self.cache.queue_glyph(0, glyph.clone());
        }
        self.texts.push(TextData {
            glyphs: glyphs.clone(),
            color,
        });
    }

    pub fn draw_text<'a>(
        &mut self,
        command_buffer: &'a mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        memory_allocator: &Arc<StandardMemoryAllocator>,
        image_num: usize,
    ) -> &'a mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer> {
        let screen_width = self.framebuffers[image_num].extent()[0];
        let screen_height = self.framebuffers[image_num].extent()[1];
        let cache_pixel_buffer = &mut self.cache_pixel_buffer;
        let cache = &mut self.cache;

        // update texture cache
        cache
            .cache_queued(|rect, src_data| {
                let width = (rect.max.x - rect.min.x) as usize;
                let height = (rect.max.y - rect.min.y) as usize;
                let mut dst_index = rect.min.y as usize * CACHE_WIDTH + rect.min.x as usize;
                let mut src_index = 0;

                for _ in 0..height {
                    let dst_slice = &mut cache_pixel_buffer[dst_index..dst_index + width];
                    let src_slice = &src_data[src_index..src_index + width];
                    dst_slice.copy_from_slice(src_slice);

                    dst_index += CACHE_WIDTH;
                    src_index += width;
                }
            })
            .unwrap();

        let buffer = CpuAccessibleBuffer::<[u8]>::from_iter(
            memory_allocator,
            BufferUsage {
                transfer_src: true,
                transfer_dst: true,
                uniform_texel_buffer: true,
                storage_texel_buffer: true,
                uniform_buffer: true,
                storage_buffer: true,
                index_buffer: true,
                vertex_buffer: true,
                indirect_buffer: true,
                shader_device_address: true,
                ..Default::default()
            },
            false,
            cache_pixel_buffer.iter().cloned(),
        )
        .unwrap();

        let (cache_texture, cache_texture_write) = ImmutableImage::uninitialized(
            memory_allocator,
            ImageDimensions::Dim2d {
                width: CACHE_WIDTH as u32,
                height: CACHE_HEIGHT as u32,
                array_layers: 1,
            },
            Format::R8_UNORM,
            1,
            ImageUsage {
                sampled: true,
                transfer_dst: true,
                ..ImageUsage::empty()
            },
            ImageCreateFlags::empty(),
            ImageLayout::General,
            Some(self.queue.queue_family_index()),
        )
        .unwrap();

        let sampler = Sampler::new(
            self.device.clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Linear,
                min_filter: Filter::Linear,
                mipmap_mode: SamplerMipmapMode::Nearest,
                address_mode: [SamplerAddressMode::Repeat; 3],
                ..Default::default()
            },
        )
        .unwrap();

        let cache_texture_view = ImageView::new_default(cache_texture).unwrap();
        let descriptor_set_allocator = StandardDescriptorSetAllocator::new(self.device.clone());

        let set = PersistentDescriptorSet::new(
            &descriptor_set_allocator,
            // self.pipeline.layout().set_layouts().get(0).unwrap().clone(),
            self.pipeline.layout().set_layouts()[0].clone(),
            [WriteDescriptorSet::image_view_sampler(
                0,
                cache_texture_view,
                sampler,
            )],
        )
        .unwrap();

        let mut clear_values = vec![Some(0f32.into())];
        let mut command_buffer = command_buffer
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                buffer,
                cache_texture_write,
            ))
            .unwrap()
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values,
                    ..RenderPassBeginInfo::framebuffer(
                        self.framebuffers[image_num as usize].clone(),
                    )
                },
                SubpassContents::Inline,
            )
            .unwrap();

        // draw
        for text in &mut self.texts.drain(..) {
            let vertices: Vec<Vertex> = text
                .glyphs
                .iter()
                .flat_map(|g| {
                    if let Ok(Some((uv_rect, screen_rect))) = cache.rect_for(0, g) {
                        let gl_rect = Rect {
                            min: point(
                                (screen_rect.min.x as f32 / screen_width as f32 - 0.5) * 2.0,
                                (screen_rect.min.y as f32 / screen_height as f32 - 0.5) * 2.0,
                            ),
                            max: point(
                                (screen_rect.max.x as f32 / screen_width as f32 - 0.5) * 2.0,
                                (screen_rect.max.y as f32 / screen_height as f32 - 0.5) * 2.0,
                            ),
                        };
                        vec![
                            // vertex 2
                            // bottom left
                            Vertex {
                                position: [gl_rect.min.x, gl_rect.max.y],
                                tex_position: [uv_rect.min.x, uv_rect.max.y],
                                color: text.color,
                            },
                            // top left
                            Vertex {
                                position: [gl_rect.min.x, gl_rect.min.y],
                                tex_position: [uv_rect.min.x, uv_rect.min.y],
                                color: text.color,
                            },
                            // vertice 1
                            // top right
                            Vertex {
                                position: [gl_rect.max.x, gl_rect.min.y],
                                tex_position: [uv_rect.max.x, uv_rect.min.y],
                                color: text.color,
                            },
                            // vertice 1
                            // top right
                            Vertex {
                                position: [gl_rect.max.x, gl_rect.min.y],
                                tex_position: [uv_rect.max.x, uv_rect.min.y],
                                color: text.color,
                            },
                            // bottom right
                            Vertex {
                                position: [gl_rect.max.x, gl_rect.max.y],
                                tex_position: [uv_rect.max.x, uv_rect.max.y],
                                color: text.color,
                            },
                            // vertex 2
                            // bottom left
                            Vertex {
                                position: [gl_rect.min.x, gl_rect.max.y],
                                tex_position: [uv_rect.min.x, uv_rect.max.y],
                                color: text.color,
                            },
                        ]
                        .into_iter()
                    } else {
                        vec![].into_iter()
                    }
                })
                .collect();

            if vertices.is_empty() {
                break;
            }

            let vertex_buffer = CpuAccessibleBuffer::from_iter(
                memory_allocator,
                BufferUsage {
                    transfer_src: true,
                    transfer_dst: true,
                    uniform_texel_buffer: true,
                    storage_texel_buffer: true,
                    uniform_buffer: true,
                    storage_buffer: true,
                    index_buffer: true,
                    vertex_buffer: true,
                    indirect_buffer: true,
                    shader_device_address: true,
                    ..Default::default()
                },
                false,
                vertices.into_iter(),
            )
            .unwrap();

            command_buffer = command_buffer
                .bind_pipeline_graphics(self.pipeline.clone())
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    self.pipeline.layout().clone(),
                    0,
                    set.clone(),
                )
                .bind_vertex_buffers(0, vertex_buffer.clone())
                .draw(vertex_buffer.len() as u32, 1, 0, 0)
                .unwrap();
        }

        command_buffer.end_render_pass().unwrap()
    }
}

impl DrawTextTrait for AutoCommandBufferBuilder<PrimaryAutoCommandBuffer> {
    fn draw_text(
        &mut self,
        data: &mut DrawText,
        memory_allocator: &Arc<StandardMemoryAllocator>,
        image_num: usize,
    ) -> &mut Self {
        data.draw_text(self, memory_allocator, image_num)
    }
}

pub trait DrawTextTrait {
    fn draw_text(
        &mut self,
        data: &mut DrawText,
        memory_allocator: &Arc<StandardMemoryAllocator>,
        image_num: usize,
    ) -> &mut Self;
}
