use std::{collections::HashMap, path::Path};

use anyhow::{anyhow, Context, Result};
use smithay::{
    backend::{
        allocator::gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
        drm::{DrmDevice, DrmDeviceFd, DrmNode},
        egl::{EGLContext, EGLDevice, EGLDisplay},
        input::{Event, InputEvent, KeyboardKeyEvent},
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        renderer::glow::GlowRenderer,
        session::{
            libseat::{LibSeatSession, LibSeatSessionNotifier},
            Session,
        },
        udev::{UdevBackend, UdevEvent},
    },
    input::keyboard::{keysyms, FilterResult},
    output::{Mode as OutputMode, Output, PhysicalProperties, Scale, Subpixel},
    reexports::{
        calloop::{
            timer::{TimeoutAction, Timer},
            LoopHandle,
        },
        drm::control::{connector, Device as ControlDevice},
        input::Libinput,
    },
    utils::Transform,
};
use tracing::{info, warn};

use crate::state::{CalloopData, EdexState};

#[derive(Debug)]
pub struct DrmDeviceState {
    pub node: DrmNode,
    pub drm: DrmDevice,
    pub gbm: GbmDevice<DrmDeviceFd>,
    pub allocator: GbmAllocator<DrmDeviceFd>,
    pub egl_display: EGLDisplay,
    pub egl_device: Option<EGLDevice>,
    pub renderer: GlowRenderer,
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
    let (session, notifier) = LibSeatSession::new().context("failed to create libseat session")?;
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
            let _ = data.display.flush_clients();
            TimeoutAction::ToDuration(std::time::Duration::from_millis(16))
        })
        .map(|_| ())
        .map_err(|error| anyhow!("failed to insert render timer: {error:?}"))
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
                                // Super+L: screen lock stub (future: ext-session-lock)
                                tracing::info!("screen lock requested (stub)");
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
    let (drm, _notifier) =
        DrmDevice::new(fd.clone(), true).context("failed to create drm device")?;
    let gbm = GbmDevice::new(fd.clone()).context("failed to create gbm device")?;
    let allocator = GbmAllocator::new(gbm.clone(), GbmBufferFlags::RENDERING);
    let egl_display =
        unsafe { EGLDisplay::new(gbm.clone()) }.context("failed to create egl display")?;
    let egl_device = EGLDevice::device_for_display(&egl_display).ok();
    let context = EGLContext::new(&egl_display).context("failed to create egl context")?;
    let renderer =
        unsafe { GlowRenderer::new(context) }.context("failed to create glow renderer")?;

    let outputs = create_outputs_for_device(display_handle, &fd, node, path);
    if backend.primary_node.is_none() {
        backend.primary_node = Some(node);
    }
    backend.outputs.extend(outputs);
    backend.devices.insert(
        device_id,
        DrmDeviceState {
            node,
            drm,
            gbm,
            allocator,
            egl_display,
            egl_device,
            renderer,
        },
    );

    info!(path = %path.display(), ?node, "initialized drm/kms device");
    Ok(())
}

pub fn on_device_removed(backend: &mut BackendState, state: &mut EdexState, device_id: u64) {
    if let Some(device) = backend.devices.remove(&device_id) {
        for output in backend.outputs.drain(..) {
            state.space.unmap_output(&output);
        }
        info!(?device.node, "removed drm device");
    }
}

fn create_outputs_for_device(
    display_handle: &smithay::reexports::wayland_server::DisplayHandle,
    fd: &DrmDeviceFd,
    node: DrmNode,
    path: &Path,
) -> Vec<Output> {
    let mut outputs = Vec::new();

    if let Ok(resources) = fd.resource_handles() {
        for connector_handle in resources.connectors() {
            let Ok(info) = fd.get_connector(*connector_handle, false) else {
                continue;
            };
            if info.state() != connector::State::Connected {
                continue;
            }

            let (physical_width, physical_height) = info.size().unwrap_or((0, 0));
            let output = Output::new(
                format!("{}-{}", info.interface().as_str(), info.interface_id()),
                PhysicalProperties {
                    size: (physical_width as i32, physical_height as i32).into(),
                    subpixel: Subpixel::Unknown,
                    make: "Unknown".into(),
                    model: path.display().to_string(),
                },
            );

            if let Some(mode) = info.modes().first() {
                let current = OutputMode {
                    size: (mode.size().0 as i32, mode.size().1 as i32).into(),
                    refresh: (mode.vrefresh() * 1000) as i32,
                };
                output.change_current_state(
                    Some(current),
                    Some(Transform::Normal),
                    Some(Scale::Integer(1)),
                    Some((0, 0).into()),
                );
                output.set_preferred(current);
            }

            output.create_global::<EdexState>(display_handle);
            outputs.push(output);
        }
    }

    if outputs.is_empty() {
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
        outputs.push(output);
    }

    outputs
}
