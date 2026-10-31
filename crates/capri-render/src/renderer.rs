use crate::Frame;

/// Las capas WGSL, en orden. Cada archivo = una rama del árbol (o FX).
const SHADER: &str = concat!(
    include_str!("../shaders/common.wgsl"),
    include_str!("../shaders/silhouette.wgsl"),
    include_str!("../shaders/extensions.wgsl"),
    include_str!("../shaders/face.wgsl"),
    include_str!("../shaders/fx.wgsl"),
    include_str!("../shaders/main.wgsl"),
);

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    premultiplied: bool,
}

impl Renderer {
    /// `target` suele ser un `Arc<winit::window::Window>`; el render no depende de winit.
    pub fn new(target: impl Into<wgpu::SurfaceTarget<'static>>, width: u32, height: u32) -> Result<Self, String> {
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        // En Windows solo DX12 + DirectComposition da ventanas transparentes.
        // En macOS (Metal) y Linux (Vulkan) la transparencia sale de la superficie.
        // Todo se puede forzar con WGPU_BACKEND / WGPU_DX12_PRESENTATION_SYSTEM.
        desc.backends = wgpu::Backends::from_env()
            .unwrap_or(if cfg!(windows) { wgpu::Backends::DX12 } else { wgpu::Backends::PRIMARY });
        desc.backend_options.dx12.presentation_system = wgpu::Dx12SwapchainKind::DxgiFromVisual;
        desc.backend_options = desc.backend_options.with_env();
        let instance = wgpu::Instance::new(desc);

        let surface = instance.create_surface(target).map_err(|e| format!("superficie: {e}"))?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .map_err(|e| format!("adaptador: {e}"))?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("capri"),
            required_limits: wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
            ..Default::default()
        }))
        .map_err(|e| format!("dispositivo: {e}"))?;

        let caps = surface.get_capabilities(&adapter);
        let mut config = surface
            .get_default_config(&adapter, width.max(1), height.max(1))
            .ok_or("la superficie no es compatible con el adaptador")?;
        // Formato sin sRGB: los colores hex del preset salen tal cual.
        config.format = caps.formats.iter().copied().find(|f| !f.is_srgb()).unwrap_or(config.format);
        config.present_mode = wgpu::PresentMode::Fifo;
        use wgpu::CompositeAlphaMode as A;
        config.alpha_mode = [A::PreMultiplied, A::PostMultiplied, A::Inherit]
            .into_iter()
            .find(|m| caps.alpha_modes.contains(m))
            .unwrap_or(A::Opaque);
        if config.alpha_mode == A::Opaque {
            eprintln!("[render] la plataforma no ofrece transparencia (modos: {:?}); el fondo será negro", caps.alpha_modes);
        }
        let premultiplied = config.alpha_mode != A::PostMultiplied;
        surface.configure(&device, &config);
        eprintln!(
            "[render] {} · {:?} · {:?} · alpha {:?}",
            adapter.get_info().name,
            adapter.get_info().backend,
            config.format,
            config.alpha_mode
        );

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("capri"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame"),
            size: std::mem::size_of::<Frame>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniforms.as_entire_binding() }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            ..Default::default()
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("capri"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        Ok(Self { surface, device, queue, config, pipeline, uniforms, bind_group, premultiplied })
    }

    pub fn premultiplied(&self) -> bool {
        self.premultiplied
    }

    pub fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn render(&mut self, frame: &Frame) {
        let texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            _ => return, // ocluida o timeout: se salta el frame
        };
        self.queue.write_buffer(&self.uniforms, 0, bytemuck::bytes_of(frame));
        let view = texture.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("body"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
        self.queue.present(texture);
    }
}

#[cfg(test)]
mod tests {
    /// Valida el WGSL sin GPU: un error de shader rompe el test, no la ventana.
    #[test]
    fn shader_validates() {
        let module = naga::front::wgsl::parse_str(super::SHADER).unwrap_or_else(|e| panic!("{}", e.emit_to_string(super::SHADER)));
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::empty())
            .validate(&module)
            .unwrap_or_else(|e| panic!("{}", e.emit_to_string(super::SHADER)));
    }

    #[test]
    fn frame_matches_wgsl_size() {
        // 21 vec4<f32> en `struct Frame` de common.wgsl
        assert_eq!(std::mem::size_of::<crate::Frame>(), 21 * 16);
    }
}
