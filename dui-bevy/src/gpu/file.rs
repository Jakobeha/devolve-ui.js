use std::mem::size_of;
use crate::gpu::{RuntimeGpuAdapter, GpuRenderContext, Vertex};
use std::mem::transmute;

#[derive(Debug)]
pub(crate) struct FileGpuAdapter {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_vertices: u16,
    num_indices: u32
}

pub(crate) struct FileGpuRenderContext<'a> {
    pub vertex_buffer: &'a mut wgpu::Buffer,
    pub index_buffer: &'a mut wgpu::Buffer,
    pub num_indices: &'a mut u32
}

// lib.rs
impl FileGpuAdapter {
    pub(crate) fn new(runtime: &RuntimeGpuAdapter, num_vertices: u16, num_indices: u32) -> Self {
        let device = &runtime.device;

        Self {
            vertex_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Vertex Buffer"),
                size: (num_vertices as usize * size_of::<Vertex>()) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::VERTEX,
                mapped_at_creation: false
            }),
            index_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Index Buffer"),
                size: (num_indices as usize * size_of::<u16>()) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::INDEX,
                mapped_at_creation: false
            }),
            num_vertices,
            num_indices,
        }
    }

    pub(crate) fn render(&mut self, runtime: &mut GpuRenderContext<'_>, render_impl: impl FnOnce(FileGpuRenderContext<'_>)) {
        // See https://github.com/gfx-rs/wgpu/discussions/1438
        // I have no idea what's the "efficient" way to copy data to the GPU
        let device = &runtime.gpu_adapter().device;
        self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: (self.num_vertices as usize * size_of::<Vertex>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: true
        });
        self.index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Index Buffer"),
            size: (self.num_indices as usize * size_of::<u16>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::INDEX,
            mapped_at_creation: true
        });

        let mut num_indices = self.num_indices;
        render_impl(FileGpuRenderContext {
            vertex_buffer: &mut self.vertex_buffer,
            index_buffer: &mut self.index_buffer,
            num_indices: &mut num_indices
        });
        assert!(num_indices <= self.num_indices, "can't add more indices than we initialized with");

        let render_pass = runtime.render_pass();
        // SAFETY: Lifetimes do not have to be for the full render pass, but only before `draw_indexed`
        unsafe {
            render_pass.set_vertex_buffer(0, extend_lifetime(self.vertex_buffer.slice(..)));
            render_pass.set_index_buffer(extend_lifetime(self.index_buffer.slice(..)), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..num_indices, 0, 0..1);
        }

        self.vertex_buffer.unmap();
        self.index_buffer.unmap()
    }
}

unsafe fn extend_lifetime<'a, 'b: 'a>(x: wgpu::BufferSlice<'a>) -> wgpu::BufferSlice<'b> {
    transmute(x)
}
