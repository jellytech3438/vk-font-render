mod vulkano_text;

use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents,
};
use vulkano::device::physical::PhysicalDevice;
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::device::{Device, DeviceCreateInfo, DeviceExtensions, QueueCreateInfo};
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{AttachmentImage, ImageAccess, SwapchainImage};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::DynamicState;
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass};
use vulkano::swapchain::{
    self, AcquireError, Swapchain, SwapchainCreateInfo, SwapchainCreationError,
    SwapchainPresentInfo,
};
use vulkano::sync;
use vulkano::sync::{FlushError, GpuFuture};
use vulkano::Version;
use vulkano::VulkanLibrary;

use vulkano_win::VkSurfaceBuild;

use vulkano_text::{DrawText, DrawTextTrait};

use winit::event::{DeviceEvent, Event, KeyboardInput, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

use std::borrow::Borrow;
use std::env;
use std::sync::Arc;
use std::time::Instant;

fn window_size_dependent_setup(
    allocator: &StandardMemoryAllocator,
    images: &[Arc<SwapchainImage>],
    render_pass: Arc<RenderPass>,
    viewport: &mut Viewport,
) -> Vec<Arc<Framebuffer>> {
    let dimensions = images[0].dimensions().width_height();

    let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [dimensions[0] as f32, dimensions[1] as f32],
        depth_range: 0.0..1.0,
    };

    let depth_buffer = ImageView::new_default(
        AttachmentImage::transient(allocator, dimensions, Format::D16_UNORM).unwrap(),
    )
    .unwrap();

    images
        .iter()
        .map(|image| {
            let image_view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![image_view, depth_buffer.clone()],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>()
}

fn main() {
    let lines = vec!(
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
        "Quisque nec lorem auctor, lobortis nulla congue, ultrices justo.",
        "Vivamus ultrices, elit quis porttitor dapibus, nisi odio fringilla arcu, vitae finibus odio lorem vel mi.",
        "Maecenas laoreet in metus et mollis.",
        "Nullam et velit dui.",
        "Quisque gravida a tortor eu pulvinar.",
        "Maecenas vitae quam nibh.",
        "Aenean lacus urna, pulvinar non vulputate vel, sollicitudin nec mauris.",
        "Integer lobortis lorem at gravida varius.",
        "Aliquam tristique, massa sed aliquet sagittis, risus erat fermentum quam, sit amet rhoncus lectus velit sit amet massa.",
        "Aenean sit amet augue urna.",
        "In porttitor dignissim erat, aliquet lacinia sapien molestie eu.",
        "Pellentesque ut pellentesque odio, id efficitur dui.",
        "Morbi ligula diam, consequat sed neque sed, posuere blandit libero.",
        "Etiam interdum pellentesque justo et vehicula.",
        "Mauris sagittis quis ante egestas luctus.",
        "",
        "Aliquam volutpat consequat nisl at tincidunt.",
        "Nam congue tellus ut est gravida interdum.",
        "Integer ut hendrerit purus.",
        "Vestibulum lobortis magna et finibus iaculis.",
        "Nam faucibus tortor id nibh placerat iaculis.",
        "Donec arcu arcu, eleifend sit amet ultrices a, consequat in ante.",
        "Sed accumsan velit dui, ac tempus lorem tempor at.",
        "Donec facilisis urna eu scelerisque volutpat.",
        "Nunc sed leo nulla.",
        "Mauris orci leo, ultricies a diam id, iaculis dapibus nibh.",
        "Nunc auctor purus vel lobortis viverra.",
        "Curabitur vitae mattis nulla, vitae vulputate leo.",
        "Mauris lacinia ultricies ullamcorper.",
        "Nullam ultrices augue nec commodo tristique.",
        "Ut et tellus sagittis, sodales elit et, vestibulum arcu.",
        "Cras dui arcu, consectetur in urna vel, lobortis elementum augue.",
        "",
        "Donec consequat orci ac commodo ultricies.",
        "Pellentesque mattis felis ut enim consequat feugiat.",
        "Vestibulum et congue sapien.",
        "Cras sem urna, condimentum sed hendrerit vitae, accumsan et orci.",
        "Etiam vitae finibus odio.",
        "Cras finibus sem sed ante varius, non posuere lectus sollicitudin.",
        "Nunc vestibulum odio at elit pharetra finibus.",
        "Pellentesque habitant morbi tristique senectus et netus et malesuada fames ac turpis egestas.",
        "Morbi varius pulvinar mauris et porttitor.",
        "Duis tincidunt vel nisl in convallis.",
        "Proin scelerisque libero nec eros aliquam lacinia.",
        "Phasellus mauris sem, ultrices non pharetra rutrum, molestie vitae dui.",
        "Fusce vulputate quam in maximus consectetur.",
        "Nulla at luctus ex.",
        "Curabitur pretium augue erat, in cursus dui hendrerit ut.",
        "",
        "Nulla viverra semper ligula porta consectetur.",
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
        "Etiam sit amet luctus erat, ac ultrices felis.",
        "Nunc placerat molestie luctus.",
        "Cras hendrerit lectus eget venenatis sodales.",
        "Vivamus hendrerit nulla vel magna mattis, a vehicula mauris elementum.",
        "Nunc euismod ut nisi pulvinar vulputate.",
        "Nullam ut leo eget mi aliquam interdum.",
        "Pellentesque sed nunc ac metus consectetur aliquam.",
        "Proin gravida tincidunt ex, et interdum ex tristique a.",
        "Maecenas fringilla gravida eros, eu interdum risus mattis consectetur.",
        "",
        "Fusce in malesuada risus, ultrices sollicitudin justo.",
        "Suspendisse dolor purus, tincidunt ac ultrices ac, blandit nec massa.",
        "Duis a consequat metus.",
        "Vestibulum condimentum ultrices varius.",
        "Sed nec convallis nibh.",
        "Vestibulum ante ipsum primis in faucibus orci luctus et ultrices posuere cubilia Curae; Nulla hendrerit cursus orci eu venenatis.",
        "Aenean condimentum enim vel metus pulvinar, sed elementum nulla sodales.",
        "Vivamus volutpat fermentum mauris vel mattis.",
    );

    let library = VulkanLibrary::new().unwrap();
    let required_extensions = vulkano_win::required_extensions(&library);
    let instance = Instance::new(
        library,
        InstanceCreateInfo {
            enabled_extensions: required_extensions,
            max_api_version: Some(Version::V1_2),
            ..Default::default()
        },
    )
    .unwrap();

    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::empty()
    };

    let event_loop = EventLoop::new();
    let surface = WindowBuilder::new()
        .build_vk_surface(&event_loop, instance.clone())
        .unwrap();

    let (physical_device, queue_family_index) = instance
        .enumerate_physical_devices()
        .unwrap()
        .filter(|p| p.supported_extensions().contains(&device_extensions))
        .filter_map(|p| {
            p.queue_family_properties()
                .iter()
                .enumerate()
                .position(|(i, q)| {
                    q.queue_flags.graphics && p.surface_support(i as u32, &surface).unwrap_or(false)
                })
                .map(|i| (p, i as u32))
        })
        .min_by_key(|(p, _)| match p.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            PhysicalDeviceType::Other => 4,
            _ => 5,
        })
        .expect("No suitable physical device found");

    let device_ext = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::none()
    };
    let (device, mut queues) = Device::new(
        physical_device,
        DeviceCreateInfo {
            enabled_extensions: device_extensions,
            queue_create_infos: vec![QueueCreateInfo {
                queue_family_index,
                ..Default::default()
            }],
            ..Default::default()
        },
    )
    .unwrap();
    let queue = queues.next().unwrap();
    let (mut swapchain, images) = {
        let caps = device
            .physical_device()
            .surface_capabilities(&surface, Default::default())
            .unwrap();
        let usage = caps.supported_usage_flags;
        let alpha = caps.supported_composite_alpha.iter().next().unwrap();
        let format = Some(
            device
                .physical_device()
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0,
        );

        let window = surface.object().unwrap().downcast_ref::<Window>().unwrap();
        let image_extent: [u32; 2] = window.inner_size().into();
        Swapchain::new(
            device.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: caps.min_image_count,
                image_format: format,
                image_extent,
                image_usage: usage,
                composite_alpha: alpha,
                // image_sharing: &queue,
                ..Default::default()
            },
        )
        // .sharing_mode(&queue)
        .unwrap()
    };

    // depth buffer is used to make sure
    // DrawText won't depend on specific render pass
    let render_pass = vulkano::single_pass_renderpass!(
        device.clone(),
        attachments: {
            color: {
                load: Clear,
                store: Store,
                format: swapchain.image_format(),
                samples: 1,
            },
            depth: {
                load: Clear,
                store: DontCare,
                format: Format::D16_UNORM,
                samples: 1,
            }
        },
        pass: {
            color: [color],
            depth_stencil: {depth}
        }
    )
    .unwrap();

    let command_buffer_allocator =
        StandardCommandBufferAllocator::new(device.clone(), Default::default());
    let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

    let mut draw_text = DrawText::new(device.clone(), queue.clone(), swapchain.clone(), &images);

    let window = surface.object().unwrap().downcast_ref::<Window>().unwrap();
    let (width, height): (u32, u32) = window.inner_size().into();
    let mut x = 0.0;
    let mut y = 0.0;
    let mut font_size = 15.0;
    let mut color = [1.0, 1.0, 1.0, 1.0];

    let mut viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [0.0, 0.0],
        depth_range: 0.0..1.0,
    };

    let mut framebuffers = window_size_dependent_setup(
        &memory_allocator,
        &images,
        render_pass.clone(),
        &mut viewport,
    );
    let mut recreate_swapchain = false;
    let mut previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>);

    let start = Instant::now();
    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }
        Event::WindowEvent {
            event: WindowEvent::Resized(_),
            ..
        } => {
            recreate_swapchain = true;
        }
        Event::DeviceEvent {
            device_id: id,
            event,
        } => match event {
            DeviceEvent::Key(k) => match k {
                KeyboardInput {
                    scancode,
                    state,
                    virtual_keycode,
                    modifiers,
                } => match virtual_keycode {
                    Some(winit::event::VirtualKeyCode::A) => {
                        font_size += 5.0;
                        recreate_swapchain = true;
                    }
                    Some(winit::event::VirtualKeyCode::M) => {
                        font_size -= 5.0;
                        recreate_swapchain = true;
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        },
        Event::RedrawEventsCleared => {
            previous_frame_end.as_mut().unwrap().cleanup_finished();
            if recreate_swapchain {
                let window = surface.object().unwrap().downcast_ref::<Window>().unwrap();
                let image_extent: [u32; 2] = window.inner_size().into();

                let (new_swapchain, new_images) = match swapchain.recreate(SwapchainCreateInfo {
                    image_extent,
                    ..swapchain.create_info()
                }) {
                    Ok(r) => r,
                    Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
                    Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                };

                swapchain = new_swapchain;
                framebuffers = window_size_dependent_setup(
                    &memory_allocator,
                    &new_images,
                    render_pass.clone(),
                    &mut viewport,
                );

                draw_text = DrawText::new(
                    device.clone(),
                    queue.clone(),
                    swapchain.clone(),
                    &new_images,
                );

                recreate_swapchain = false;
            }

            // render the text with position, size, color, and text itself
            for (i, line) in lines.iter().enumerate() {
                draw_text.queue_text(x, y + (i + 1) as f32 * font_size, font_size, color, line);
            }

            let (image_num, suboptimal, acquire_future) =
                match swapchain::acquire_next_image(swapchain.clone(), None) {
                    Ok(r) => r,
                    Err(AcquireError::OutOfDate) => {
                        recreate_swapchain = true;
                        return;
                    }
                    Err(e) => panic!("Failed to acquire next image: {:?}", e),
                };

            if suboptimal {
                recreate_swapchain = true;
            }

            let clear_values = vec![Some([0.0, 0.0, 0.0, 1.0].into()), Some(1f32.into())];
            let mut builder = AutoCommandBufferBuilder::primary(
                &command_buffer_allocator,
                queue.queue_family_index(),
                CommandBufferUsage::OneTimeSubmit,
            )
            .unwrap();

            builder
                .begin_render_pass(
                    RenderPassBeginInfo {
                        clear_values,
                        ..RenderPassBeginInfo::framebuffer(framebuffers[image_num as usize].clone())
                    },
                    SubpassContents::Inline,
                )
                .unwrap()
                .end_render_pass()
                .unwrap()
                .draw_text(&mut draw_text, &memory_allocator, image_num as usize);

            let command_buffer = builder.build().unwrap();

            let future = previous_frame_end
                .take()
                .unwrap()
                .join(acquire_future)
                .then_execute(queue.clone(), command_buffer)
                .unwrap()
                .then_swapchain_present(
                    queue.clone(),
                    SwapchainPresentInfo::swapchain_image_index(swapchain.clone(), image_num),
                )
                .then_signal_fence_and_flush();

            match future {
                Ok(future) => {
                    previous_frame_end = Some(Box::new(future) as Box<_>);
                }
                Err(FlushError::OutOfDate) => {
                    recreate_swapchain = true;
                    previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<_>);
                }
                Err(e) => {
                    println!("Failed to flush future: {:?}", e);
                    previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<_>);
                }
            }
        }
        _ => (),
    });
}
