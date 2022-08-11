use std::iter::once;
use log::debug;
use pollster::block_on;
use winit::window::Window;
use crate::gpu::{GpuSettings, GpuSettingsPriority, Vertex};
use derive_more::{Display, Error, From};
use winit_modular::event_loop::EventLoop;

#[derive(Debug, Display, Error, From)]
pub enum GpuSetupError {
    #[display(fmt = "no gpu found")]
    NoGpu,
    RequestDeviceError(wgpu::RequestDeviceError),
    #[display(fmt = "no supported formats")]
    NoSupportedFormats
}

#[doc(hidden)]
pub struct GpuRenderContext<'a> {
    gpu_adapter: *mut RuntimeGpuAdapter,
    render_pass: *mut wgpu::RenderPass<'a>,
}

impl<'a> GpuRenderContext<'a> {
    pub(crate) fn gpu_adapter(&mut self) -> &mut RuntimeGpuAdapter {
        // SAFETY: Has a reference, only when `render_pass` is not also dereferenced.
        unsafe { &mut *self.gpu_adapter }
    }

    pub(crate) fn render_pass(&mut self) -> &mut wgpu::RenderPass<'a> {
        // SAFETY: Has a reference, only when `gpu_adapter` is not also dereferenced.
        unsafe { &mut *self.render_pass }
    }
}

pub(crate) struct RuntimeGpuAdapter {
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface,
    pub(super) device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    // May be per-file later, for now we only have one shader
    pub(super) render_pipeline: wgpu::RenderPipeline,
    shader: wgpu::ShaderModule
}

struct SendPtr<T>(pub *const T);

unsafe impl<T: Send> Send for SendPtr<T> {}

impl<T> SendPtr<T> {
    unsafe fn deref(&self) -> &T {
        &*self.0
    }
}

// lib.rs
impl RuntimeGpuAdapter {
    pub(crate) fn try_new(event_loop: &EventLoop, window: &Window, settings: &GpuSettings) -> Result<Self, GpuSetupError> {
        // This is called on a separate thread so we can just completely ignore async
        block_on(Self::try_new_async(event_loop, window, settings))
    }

    async fn try_new_async(event_loop: &EventLoop, window: &Window, settings: &GpuSettings) -> Result<Self, GpuSetupError> {
        let size = window.inner_size();

        // GPU configuration

        // This part is from bevy...
        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(settings.backends);

        // SAFETY: We `await` so the pointers must be live
        let surface = event_loop.on_main_thread::<(&wgpu::Instance, &Window), _>(move |(instance, window)| {
            unsafe { instance.create_surface(&window) }
        }, (&instance, window)).await;

        let request_adapter_options = wgpu::RequestAdapterOptions {
            power_preference: settings.power_preference,
            compatible_surface: Some(&surface),
            ..Default::default()
        };
        let (device, queue, adapter) = Self::initialize_renderer(&instance, &settings, &request_adapter_options).await?;
        debug!("Configured wgpu adapter Limits: {:#?}", device.limits());
        debug!("Configured wgpu adapter Features: {:#?}", device.features());

        // This part is from learn-wgpu...
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&adapter).into_iter().next().ok_or(GpuSetupError::NoSupportedFormats)?,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        surface.configure(&device, &config);
        // Done GPU configuration

        // Global but may be per-file later
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });


        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
        });

        Ok(Self {
            size,
            surface,
            device,
            queue,
            config,
            render_pipeline,
            shader
        })
    }

    /// Initializes the renderer by retrieving and preparing the GPU instance, device and queue
    /// for the specified backend.
    async fn initialize_renderer(
        instance: &wgpu::Instance,
        settings: &GpuSettings,
        request_adapter_options: &wgpu::RequestAdapterOptions<'_>,
    ) -> Result<(wgpu::Device, wgpu::Queue, wgpu::Adapter), GpuSetupError> {
        let adapter = instance
            .request_adapter(request_adapter_options)
            .await
            .ok_or(GpuSetupError::NoGpu)?;

        let adapter_info = adapter.get_info();
        debug!("adapter info: {:?}", adapter_info);

        let trace_path = if cfg!(feature = "wgpu_trace") {
            let path = std::path::Path::new("wgpu_trace");
            // ignore potential error, wgpu will log it
            let _ = std::fs::create_dir(path);
            Some(path)
        } else {
            None
        };

        // Maybe get features and limits based on what is supported by the adapter/backend
        let mut features = wgpu::Features::empty();
        let mut limits = settings.limits.clone();
        if matches!(settings.priority, GpuSettingsPriority::Functionality) {
            features = adapter.features() | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES;
            if adapter_info.device_type == wgpu::DeviceType::DiscreteGpu {
                // `MAPPABLE_PRIMARY_BUFFERS` can have a significant, negative performance impact for
                // discrete GPUs due to having to transfer data across the PCI-E bus and so it
                // should not be automatically enabled in this case. It is however beneficial for
                // integrated GPUs.
                features -= wgpu::Features::MAPPABLE_PRIMARY_BUFFERS;
            }
            limits = adapter.limits();
        }

        // Enforce the disabled features
        if let Some(disabled_features) = settings.disabled_features {
            features -= disabled_features;
        }
        // NOTE: |= is used here to ensure that any explicitly-enabled features are respected.
        features |= settings.features;

        // Enforce the limit constraints
        if let Some(constrained_limits) = settings.constrained_limits.as_ref() {
            // NOTE: Respect the configured limits as an 'upper bound'. This means for 'max' limits, we
            // take the minimum of the calculated limits according to the adapter/backend and the
            // specified max_limits. For 'min' limits, take the maximum instead. This is intended to
            // err on the side of being conservative. We can't claim 'higher' limits that are supported
            // but we can constrain to 'lower' limits.
            limits = wgpu::Limits {
                max_texture_dimension_1d: limits
                    .max_texture_dimension_1d
                    .min(constrained_limits.max_texture_dimension_1d),
                max_texture_dimension_2d: limits
                    .max_texture_dimension_2d
                    .min(constrained_limits.max_texture_dimension_2d),
                max_texture_dimension_3d: limits
                    .max_texture_dimension_3d
                    .min(constrained_limits.max_texture_dimension_3d),
                max_texture_array_layers: limits
                    .max_texture_array_layers
                    .min(constrained_limits.max_texture_array_layers),
                max_bind_groups: limits
                    .max_bind_groups
                    .min(constrained_limits.max_bind_groups),
                max_dynamic_uniform_buffers_per_pipeline_layout: limits
                    .max_dynamic_uniform_buffers_per_pipeline_layout
                    .min(constrained_limits.max_dynamic_uniform_buffers_per_pipeline_layout),
                max_dynamic_storage_buffers_per_pipeline_layout: limits
                    .max_dynamic_storage_buffers_per_pipeline_layout
                    .min(constrained_limits.max_dynamic_storage_buffers_per_pipeline_layout),
                max_sampled_textures_per_shader_stage: limits
                    .max_sampled_textures_per_shader_stage
                    .min(constrained_limits.max_sampled_textures_per_shader_stage),
                max_samplers_per_shader_stage: limits
                    .max_samplers_per_shader_stage
                    .min(constrained_limits.max_samplers_per_shader_stage),
                max_storage_buffers_per_shader_stage: limits
                    .max_storage_buffers_per_shader_stage
                    .min(constrained_limits.max_storage_buffers_per_shader_stage),
                max_storage_textures_per_shader_stage: limits
                    .max_storage_textures_per_shader_stage
                    .min(constrained_limits.max_storage_textures_per_shader_stage),
                max_uniform_buffers_per_shader_stage: limits
                    .max_uniform_buffers_per_shader_stage
                    .min(constrained_limits.max_uniform_buffers_per_shader_stage),
                max_uniform_buffer_binding_size: limits
                    .max_uniform_buffer_binding_size
                    .min(constrained_limits.max_uniform_buffer_binding_size),
                max_storage_buffer_binding_size: limits
                    .max_storage_buffer_binding_size
                    .min(constrained_limits.max_storage_buffer_binding_size),
                max_vertex_buffers: limits
                    .max_vertex_buffers
                    .min(constrained_limits.max_vertex_buffers),
                max_vertex_attributes: limits
                    .max_vertex_attributes
                    .min(constrained_limits.max_vertex_attributes),
                max_vertex_buffer_array_stride: limits
                    .max_vertex_buffer_array_stride
                    .min(constrained_limits.max_vertex_buffer_array_stride),
                max_push_constant_size: limits
                    .max_push_constant_size
                    .min(constrained_limits.max_push_constant_size),
                min_uniform_buffer_offset_alignment: limits
                    .min_uniform_buffer_offset_alignment
                    .max(constrained_limits.min_uniform_buffer_offset_alignment),
                min_storage_buffer_offset_alignment: limits
                    .min_storage_buffer_offset_alignment
                    .max(constrained_limits.min_storage_buffer_offset_alignment),
                max_inter_stage_shader_components: limits
                    .max_inter_stage_shader_components
                    .min(constrained_limits.max_inter_stage_shader_components),
                max_compute_workgroup_storage_size: limits
                    .max_compute_workgroup_storage_size
                    .min(constrained_limits.max_compute_workgroup_storage_size),
                max_compute_invocations_per_workgroup: limits
                    .max_compute_invocations_per_workgroup
                    .min(constrained_limits.max_compute_invocations_per_workgroup),
                max_compute_workgroup_size_x: limits
                    .max_compute_workgroup_size_x
                    .min(constrained_limits.max_compute_workgroup_size_x),
                max_compute_workgroup_size_y: limits
                    .max_compute_workgroup_size_y
                    .min(constrained_limits.max_compute_workgroup_size_y),
                max_compute_workgroup_size_z: limits
                    .max_compute_workgroup_size_z
                    .min(constrained_limits.max_compute_workgroup_size_z),
                max_compute_workgroups_per_dimension: limits
                    .max_compute_workgroups_per_dimension
                    .min(constrained_limits.max_compute_workgroups_per_dimension),
                max_buffer_size: limits
                    .max_buffer_size
                    .min(constrained_limits.max_buffer_size),
            };
        }

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: settings.device_label.as_ref().map(|a| a.as_ref()),
                features,
                limits,
            },
            trace_path,
        ).await?;
        Ok((device, queue, adapter))
    }

    pub(crate) fn rerender(&mut self, render_impl: impl FnOnce(GpuRenderContext<'_>)) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let texture_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut command_encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let self_ptr = self as *mut _;
            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render pass"),
                // Clear background
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.2, b: 0.3, a: 1.0 }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);

            render_impl(GpuRenderContext {
                gpu_adapter: self_ptr,
                render_pass: &mut render_pass as *mut _
            });
        }

        self.queue.submit(once(command_encoder.finish()));
        output.present();

        Ok(())
    }
}
