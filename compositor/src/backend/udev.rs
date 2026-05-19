use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use anyhow::{anyhow, Context, Result};
use smithay::{
    backend::{
        allocator::{
            gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
            Fourcc,
        },
        drm::{
            compositor::{DrmCompositor, FrameFlags},
            exporter::gbm::GbmFramebufferExporter,
            DrmDevice, DrmDeviceFd, DrmNode,
        },
        egl::{EGLContext, EGLDisplay},
        input::{Event, InputEvent, KeyboardKeyEvent},
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        renderer::{
            element::{surface::WaylandSurfaceRenderElement, AsRenderElements},
            glow::GlowRenderer,
            ImportDma,
        },
        session::{
            libseat::{LibSeatSession, LibSeatSessionNotifier},
            Session,
        },
        udev::{UdevBackend, UdevEvent},
    },
    desktop::layer_map_for_output,
    input::keyboard::{keysyms, FilterResult},
    output::{Mode as OutputMode, Output, PhysicalProperties, Scale, Subpixel},
    reexports::{
        calloop::{
            timer::{TimeoutAction, Timer},
            LoopHandle,
        },
        drm::control::{connector, crtc, Device as ControlDevice},
        input::Libinput,
    },
    utils::Transform,
};
use tracing::{info, warn};

use crate::state::{CalloopData, EdexState};

/// Per-output DRM compositing state.
pub struct OutputCompositorState {
    pub output: Output,
    pub compositor: DrmCompositor<
        GbmAllocator<DrmDeviceFd>,
        GbmFramebufferExporter<DrmDeviceFd>,
        (),
        DrmDeviceFd,
    >,
}

impl std::fmt::Debug for OutputCompositorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OutputCompositorState")
            .field("output", &self.output.name())
            .finish()
    }
}

#[derive(Debug)]
pub struct DrmDeviceState {
    pub node: DrmNode,
    pub drm: DrmDevice,
    pub gbm: GbmDevice<DrmDeviceFd>,
    pub renderer: GlowRenderer,
    pub compositors: Vec<OutputCompositorState>,
}

#[derive(Debug)]
pub struct BackendState {
    pub session: LibSeatSession,
    pub seat_name: String,
    pub primary_node: Option<DrmNode>,
    pub devices: HashMap<u64, DrmDeviceState>,
    pub outputs: Vec<Output>,
}

pub fn init_udev_backend(
    state: &mut EdexState,
    handle: &LoopHandle<'static, CalloopData>,
) -> Result<BackendState> {
    let (session, notifier) = LibSeatSession::new().map_err(|e| {
        let msg = format!(
            "Failed to create libseat session: {e}\n\
            \n\
            This usually means your user does not have permission to access DRM/KMS devices.\n\
            Fix: add yourself to the seat, input, and video groups, then log out and back in:\n\
            \n\
            \tsudo usermod -aG seat,input,video $USER\n\
            \n\
            Also ensure systemd-logind is running:\n\
            \n\
            \tsystemctl status systemd-logind\n\
            \n\
            Session log: ~/.local/share/edex-de/session.log"
        );
        tracing::error!("{}", msg);
        anyhow!("{}", msg)
    })?;
    let seat_name = session.seat();
    let udev = UdevBackend::new(&seat_name).context("failed to create udev backend")?;

    let mut backend = BackendState {
        session: session.clone(),
        seat_name: seat_name.clone(),
        primary_node: None,
        devices: HashMap::new(),
        outputs: Vec::new(),
    };

    for (device_id, path) in udev.device_list() {
        on_device_added(&mut backend, &state.display_handle, device_id, path)?;
    }

    install_session_notifier(handle, notifier)?;
    install_udev_source(handle, udev)?;
    install_libinput_source(handle, session.clone(), seat_name.clone())?;
    install_render_tick(handle)?;

    state.seat_name = seat_name;

    Ok(backend)
}

fn install_session_notifier(
    handle: &LoopHandle<'static, CalloopData>,
    notifier: LibSeatSessionNotifier,
) -> Result<()> {
    handle
        .insert_source(notifier, |event, _, data| {
            info!(?event, "libseat session event");
            let _ = data.display.flush_clients();
        })
        .map(|_| ())
        .map_err(|error| anyhow!("failed to insert libseat notifier: {error:?}"))
}

fn install_udev_source(handle: &LoopHandle<'static, CalloopData>, udev: UdevBackend) -> Result<()> {
    handle
        .insert_source(udev, |event, _, data| match event {
            UdevEvent::Added { device_id, path } => {
                if let Err(error) = on_device_added(
                    &mut data.backend,
                    &data.state.display_handle,
                    device_id,
                    &path,
                ) {
                    warn!(?error, path = %path.display(), "failed to initialize drm device");
                }
            }
            UdevEvent::Removed { device_id } => {
                on_device_removed(&mut data.backend, &mut data.state, device_id);
            }
            UdevEvent::Changed { device_id } => {
                info!(device_id, "udev device changed");
            }
        })
        .map(|_| ())
        .map_err(|error| anyhow!("failed to insert udev source: {error:?}"))
}

fn install_libinput_source(
    handle: &LoopHandle<'static, CalloopData>,
    session: LibSeatSession,
    seat_name: String,
) -> Result<()> {
    let mut context = Libinput::new_with_udev(LibinputSessionInterface::from(session));
    context
        .udev_assign_seat(&seat_name)
        .map_err(|_| anyhow::anyhow!("failed to assign libinput seat {seat_name}"))?;
    let backend = LibinputInputBackend::new(context);

    handle
        .insert_source(backend, |event, _, data| {
            dispatch_input_event(event, &mut data.state)
        })
        .map(|_| ())
        .map_err(|error| anyhow!("failed to insert libinput source: {error:?}"))
}

fn install_render_tick(handle: &LoopHandle<'static, CalloopData>) -> Result<()> {
    handle
        .insert_source(Timer::immediate(), |_, _, data| {
            render_outputs(&mut data.backend);
            let _ = data.display.flush_clients();
            TimeoutAction::ToDuration(std::time::Duration::from_millis(16))
        })
        .map(|_| ())
        .map_err(|error| anyhow!("failed to insert render timer: {error:?}"))
}

/// Render all DRM outputs. Called every ~16ms from the event loop timer.
pub fn render_outputs(backend: &mut BackendState) {
    for device in backend.devices.values_mut() {
        let renderer = &mut device.renderer;
        let compositors = &mut device.compositors;
        render_device(renderer, compositors);
    }
}

fn render_device(renderer: &mut GlowRenderer, compositors: &mut [OutputCompositorState]) {
    // eDEX dark-blue background — visible even before any client connects.
    const CLEAR_COLOR: [f32; 4] = [0.02, 0.04, 0.08, 1.0];

    for cs in compositors.iter_mut() {
        // Collect layer-shell surfaces for this output.
        let elements: Vec<WaylandSurfaceRenderElement<GlowRenderer>> = {
            let layer_map = layer_map_for_output(&cs.output);
            let scale: smithay::utils::Scale<f64> = smithay::utils::Scale::from(1.0f64);
            layer_map
                .layers()
                .flat_map(|surface| {
                    AsRenderElements::<GlowRenderer>::render_elements::<
                        WaylandSurfaceRenderElement<GlowRenderer>,
                    >(surface, renderer, (0, 0).into(), scale, 1.0f32)
                })
                .collect()
        };

        match cs
            .compositor
            .render_frame::<GlowRenderer, WaylandSurfaceRenderElement<GlowRenderer>>(
                renderer,
                &elements,
                CLEAR_COLOR,
                FrameFlags::DEFAULT,
            ) {
            Ok(result) if !result.is_empty => {
                if let Err(e) = cs.compositor.commit_frame() {
                    warn!("DRM commit_frame error on {}: {}", cs.output.name(), e);
                    cs.compositor.reset_buffers();
                }
            }
            Ok(_) => {
                // No damage — force redraw next tick so the screen is never stuck.
                cs.compositor.reset_buffers();
            }
            Err(e) => {
                warn!("render_frame error on {}: {}", cs.output.name(), e);
                cs.compositor.reset_buffers();
            }
        }
    }
}

fn dispatch_input_event(event: InputEvent<LibinputInputBackend>, state: &mut EdexState) {
    match event {
        InputEvent::Keyboard { event } => {
            if let Some(keyboard) = state.seat.get_keyboard() {
                let serial = smithay::utils::SERIAL_COUNTER.next_serial();
                let pressed = matches!(event.state(), smithay::backend::input::KeyState::Pressed);
                keyboard.input::<(), _>(
                    state,
                    event.key_code(),
                    event.state(),
                    serial,
                    event.time_msec(),
                    |data, modifiers, handle| {
                        if !pressed {
                            return FilterResult::Forward;
                        }

                        let sym = handle
                            .raw_latin_sym_or_raw_current_sym()
                            .unwrap_or_else(|| handle.modified_sym());

                        if modifiers.alt && sym == keysyms::KEY_space.into() {
                            if let Ok(mut open) = data.launcher_open.lock() {
                                *open = true;
                            }
                            return FilterResult::Intercept(());
                        }

                        if modifiers.alt && sym == keysyms::KEY_Tab.into() {
                            data.cycle_focus();
                            return FilterResult::Intercept(());
                        }

                        if modifiers.logo {
                            if matches!(sym, s if s == keysyms::KEY_q.into() || s == keysyms::KEY_Q.into()) {
                                data.close_focused_window();
                                return FilterResult::Intercept(());
                            }

                            if sym == keysyms::KEY_comma.into() {
                                if let Ok(mut open) = data.settings_open.lock() {
                                    *open = !*open;
                                }
                                return FilterResult::Intercept(());
                            }

                            if sym == keysyms::KEY_l.into() || sym == keysyms::KEY_L.into() {
                                data.lock_screen = true;
                                tracing::info!("screen lock requested");
                                return FilterResult::Intercept(());
                            }

                            let workspace = if sym == keysyms::KEY_1.into() {
                                Some(1)
                            } else if sym == keysyms::KEY_2.into() {
                                Some(2)
                            } else if sym == keysyms::KEY_3.into() {
                                Some(3)
                            } else if sym == keysyms::KEY_4.into() {
                                Some(4)
                            } else if sym == keysyms::KEY_5.into() {
                                Some(5)
                            } else if sym == keysyms::KEY_6.into() {
                                Some(6)
                            } else if sym == keysyms::KEY_7.into() {
                                Some(7)
                            } else if sym == keysyms::KEY_8.into() {
                                Some(8)
                            } else if sym == keysyms::KEY_9.into() {
                                Some(9)
                            } else {
                                None
                            };
                            if let Some(workspace) = workspace {
                                info!(workspace, "workspace switch stub");
                                return FilterResult::Intercept(());
                            }
                        }

                        FilterResult::Forward
                    },
                );
            }
        }
        InputEvent::PointerMotion { .. }
        | InputEvent::PointerMotionAbsolute { .. }
        | InputEvent::PointerButton { .. }
        | InputEvent::PointerAxis { .. } => {}
        _ => {}
    }
}

pub fn on_device_added(
    backend: &mut BackendState,
    display_handle: &smithay::reexports::wayland_server::DisplayHandle,
    device_id: u64,
    path: &Path,
) -> Result<()> {
    if backend.devices.contains_key(&device_id) {
        return Ok(());
    }

    let node = DrmNode::from_path(path)
        .with_context(|| format!("failed to derive drm node from {}", path.display()))?;

    let opened = backend
        .session
        .open(path, smithay::reexports::rustix::fs::OFlags::RDWR)
        .with_context(|| {
            format!(
                "failed to open drm device {} through libseat",
                path.display()
            )
        })?;
    let fd = DrmDeviceFd::new(opened.into());

    let (mut drm, _notifier) =
        DrmDevice::new(fd.clone(), true).context("failed to create drm device")?;
    let gbm = GbmDevice::new(fd.clone()).context("failed to create gbm device")?;

    // Build the OpenGL renderer from an EGL context on this GBM device.
    let egl_display =
        unsafe { EGLDisplay::new(gbm.clone()) }.context("failed to create egl display")?;
    let context = EGLContext::new(&egl_display).context("failed to create egl context")?;
    let renderer =
        unsafe { GlowRenderer::new(context) }.context("failed to create glow renderer")?;

    // Collect the renderer's supported dmabuf formats — needed by DrmCompositor.
    let renderer_formats = renderer
        .dmabuf_formats()
        .into_iter()
        .collect::<HashSet<_>>();

    // Find and set up one DrmCompositor per connected connector.
    let mut compositors: Vec<OutputCompositorState> = Vec::new();
    let mut used_crtcs: HashSet<crtc::Handle> = HashSet::new();
    let mut all_outputs: Vec<Output> = Vec::new();

    if let Ok(resources) = fd.resource_handles() {
        for connector_handle in resources.connectors() {
            let Ok(connector_info) = fd.get_connector(*connector_handle, false) else {
                continue;
            };
            if connector_info.state() != connector::State::Connected {
                continue;
            }
            let Some(&mode) = connector_info.modes().first() else {
                continue;
            };

            let Some(crtc_handle) = find_crtc(&drm, &resources, &connector_info, &used_crtcs)
            else {
                warn!(
                    "no available CRTC for connector {:?}-{}",
                    connector_info.interface(),
                    connector_info.interface_id()
                );
                continue;
            };
            used_crtcs.insert(crtc_handle);

            let drm_surface = match drm.create_surface(crtc_handle, mode, &[*connector_handle]) {
                Ok(s) => s,
                Err(e) => {
                    warn!("failed to create DRM surface for connector: {}", e);
                    continue;
                }
            };

            let (phys_w, phys_h) = connector_info.size().unwrap_or((0, 0));
            let output = Output::new(
                format!(
                    "{}-{}",
                    connector_info.interface().as_str(),
                    connector_info.interface_id()
                ),
                PhysicalProperties {
                    size: (phys_w as i32, phys_h as i32).into(),
                    subpixel: Subpixel::Unknown,
                    make: "Unknown".into(),
                    model: path.display().to_string(),
                },
            );
            let current_mode = OutputMode {
                size: (mode.size().0 as i32, mode.size().1 as i32).into(),
                refresh: (mode.vrefresh() * 1000) as i32,
            };
            output.change_current_state(
                Some(current_mode),
                Some(Transform::Normal),
                Some(Scale::Integer(1)),
                Some((0, 0).into()),
            );
            output.set_preferred(current_mode);
            output.create_global::<EdexState>(display_handle);

            let allocator = GbmAllocator::new(
                gbm.clone(),
                GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT,
            );
            let exporter = GbmFramebufferExporter::<DrmDeviceFd>::new(gbm.clone(), Some(node));

            match DrmCompositor::new(
                &output,
                drm_surface,
                None,
                allocator,
                exporter,
                [Fourcc::Argb8888, Fourcc::Xrgb8888],
                renderer_formats.clone(),
                drm.cursor_size(),
                Some(gbm.clone()),
            ) {
                Ok(compositor) => {
                    info!(
                        output = output.name(),
                        mode = ?current_mode,
                        "initialized DRM compositor for output"
                    );
                    all_outputs.push(output.clone());
                    compositors.push(OutputCompositorState { output, compositor });
                }
                Err(e) => {
                    warn!(
                        "failed to create DRM compositor for {}: {}",
                        output.name(),
                        e
                    );
                }
            }
        }
    }

    if compositors.is_empty() {
        // Fallback: create a synthetic output so the session has *something*.
        warn!(
            path = %path.display(),
            "no connectors initialized; creating synthetic fallback output"
        );
        let output = Output::new(
            format!("{}-fallback", node),
            PhysicalProperties {
                size: (0, 0).into(),
                subpixel: Subpixel::Unknown,
                make: "Generic".into(),
                model: path.display().to_string(),
            },
        );
        let mode = OutputMode {
            size: (1920, 1080).into(),
            refresh: 60_000,
        };
        output.change_current_state(
            Some(mode),
            Some(Transform::Normal),
            Some(Scale::Integer(1)),
            Some((0, 0).into()),
        );
        output.set_preferred(mode);
        output.create_global::<EdexState>(display_handle);
        all_outputs.push(output);
    }

    if backend.primary_node.is_none() {
        backend.primary_node = Some(node);
    }
    backend.outputs.extend(all_outputs);
    backend.devices.insert(
        device_id,
        DrmDeviceState {
            node,
            drm,
            gbm,
            renderer,
            compositors,
        },
    );

    info!(path = %path.display(), ?node, "initialized drm/kms device");
    Ok(())
}

/// Find an available CRTC for a connected connector.
fn find_crtc(
    drm: &DrmDevice,
    resources: &smithay::reexports::drm::control::ResourceHandles,
    connector: &smithay::reexports::drm::control::connector::Info,
    used_crtcs: &HashSet<crtc::Handle>,
) -> Option<crtc::Handle> {
    for enc_handle in connector.encoders().iter() {
        let Ok(enc) = drm.get_encoder(*enc_handle) else {
            continue;
        };
        // Prefer the encoder's currently active CRTC.
        if let Some(c) = enc.crtc() {
            if !used_crtcs.contains(&c) {
                return Some(c);
            }
        }
        // Fall back to any CRTC compatible with this encoder.
        for c in resources.filter_crtcs(enc.possible_crtcs()) {
            if !used_crtcs.contains(&c) {
                return Some(c);
            }
        }
    }
    None
}

pub fn on_device_removed(backend: &mut BackendState, state: &mut EdexState, device_id: u64) {
    if let Some(device) = backend.devices.remove(&device_id) {
        for cs in &device.compositors {
            state.space.unmap_output(&cs.output);
        }
        backend.outputs.retain(|o| {
            !device
                .compositors
                .iter()
                .any(|cs| cs.output.name() == o.name())
        });
        info!(?device.node, "removed drm device");
    }
}
