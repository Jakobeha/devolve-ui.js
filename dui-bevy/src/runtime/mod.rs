//! This code was copied and modified from [learn-wgpu](https://sotrh.github.io/learn-wgpu/) and
//! [bevy-render](https://github.com/bevyengine/bevy/blob/latest/crates/bevy_render/)

use std::time::{Duration, Instant};
use winit::error::OsError;
use devolve_ui_core::dui_impl::{ControlFlow, DuiRenderError, DuiRuntime};
use winit::window::{Window, WindowBuilder};
use derive_more::{Display, Error, From};
use log::error;
use pollster::block_on;
use winit_modular::event_loop::EventLoop;
use crate::gpu::{RuntimeGpuAdapter, GpuRenderContext, GpuSetupError, GpuSettings};

pub struct Runtime {
    event_loop: EventLoop,
    window: Window,
    pub(crate) gpu: RuntimeGpuAdapter
}

#[derive(Debug, Display, Error, From)]
pub enum Error {
    Os(OsError),
    Io(std::io::Error),
    #[display(fmt = "error setting up wgpu: {}", _0)]
    GpuInit(GpuSetupError)
}

impl Runtime {
    pub fn try_new(configure_window: impl FnOnce(WindowBuilder) -> WindowBuilder + Send + 'static) -> Result<Self, Error> {
        block_on(Self::try_new_async(configure_window))
    }

    pub async fn try_new_async(configure_window: impl FnOnce(WindowBuilder) -> WindowBuilder + Send + 'static) -> Result<Self, Error> {
        // From https://github.com/sotrh/learn-wgpu/blob/master/code/intermediate/tutorial10-lighting/src/lib.rs
        let event_loop = EventLoop::new().await;
        let window = event_loop.create_window(configure_window).await?;

        #[cfg(target_arch = "wasm32")]
        {
            // Winit prevents sizing with CSS, so we have to set
            // the size manually when on web.
            use winit::dpi::PhysicalSize;
            use winit::platform::web::WindowExtWebSys;

            window.set_inner_size(PhysicalSize::new(450, 400));

            web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| {
                    let canvas = web_sys::Element::from(window.canvas());
                    if let Some(dst) = doc.get_element_by_id("wasm-example") {
                        dst.append_child(&canvas).ok()?;
                    } else {
                        doc.body()?.append_child(&canvas).ok()?;
                    }
                    Some(())
                })?;
        }

        let gpu_state = RuntimeGpuAdapter::try_new(&event_loop, &window, &GpuSettings::default())?;

        Ok(Runtime {
            event_loop,
            window,
            gpu: gpu_state
        })
    }

    fn control_flow_after_render_error() -> winit_modular::event_loop::ControlFlow {
        // wait a bit so we don't spam errors, but don't exit
        winit_modular::event_loop::ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(5000))
    }
}

impl DuiRuntime for Runtime {
    type Context<'a> = GpuRenderContext<'a>;

    fn run(self, mut rerender: impl FnMut(Self::Context<'_>, &mut ControlFlow) -> Result<(), DuiRenderError> + 'static) {
        let mut gpu = self.gpu;
        self.event_loop.run(move |event, control_flow, _is_pending| {
            match event {
                winit_modular::event::Event::WindowEvent { event, .. } => match event {
                    winit_modular::event::WindowEvent::CloseRequested => *control_flow = winit_modular::event_loop::ControlFlow::ExitLocal,
                    _ => {}
                },
                _ => {}
            }
            let mut my_control_flow = winit_to_control_flow(*control_flow);
            if let Err(err) = gpu.rerender(|gpu| {
                if let Err(err) = rerender(gpu, &mut my_control_flow) {
                    error!("error rendering (1): {}", err);
                    *control_flow = Self::control_flow_after_render_error();
                }
            }) {
                error!("error rendering (2): {}", err);
                *control_flow = Self::control_flow_after_render_error();
            } else {
                *control_flow = control_flow_to_winit(my_control_flow);
            }
        })
    }
}

fn winit_to_control_flow(control_flow: winit_modular::event_loop::ControlFlow) -> ControlFlow {
    match control_flow {
        winit_modular::event_loop::ControlFlow::Poll => ControlFlow::Continue,
        winit_modular::event_loop::ControlFlow::Wait => ControlFlow::Continue,
        winit_modular::event_loop::ControlFlow::WaitUntil(_) => ControlFlow::Continue,
        winit_modular::event_loop::ControlFlow::ExitLocal => ControlFlow::Stop,
        winit_modular::event_loop::ControlFlow::ExitApp => ControlFlow::Stop,
        // winit_modular::event_loop::ControlFlow::ExitWithCode(_) => ControlFlow::Stop winit 0.27
    }
}

fn control_flow_to_winit(control_flow: ControlFlow) -> winit_modular::event_loop::ControlFlow {
    match control_flow {
        ControlFlow::Continue => winit_modular::event_loop::ControlFlow::Poll,
        ControlFlow::Stop => winit_modular::event_loop::ControlFlow::ExitLocal,
        // ControlFlow::Stop => winit_modular::event_loop::ControlFlow::ExitWithCode(0) winit 0.27
    }
}