use std::{sync::Arc, error::Error};

use eyre::Result;

use smithay::{
    delegate_shm,
    delegate_seat,
    delegate_xdg_shell,
    delegate_compositor,
};
use smithay::wayland::{
    shm::ShmState,
    buffer::{Buffer,BufferHandler},
    seat::{
        Seat,
        SeatState,
        SeatHandler,
        FilterResult
    },
    shell::xdg::{
        XdgRequest,
        XdgShellState,
        XdgShellHandler
    },
    compositor::{
        TraversalAction,
        CompositorState,
        CompositorHandler,
        SurfaceAttributes,
        with_surface_tree_downward,
    },
};
use smithay::backend::{
    renderer::{
        Frame, Renderer,
        utils::{draw_surface_tree, on_commit_buffer_handler},
    },
    winit::{self, WinitEvent},
    input::{InputEvent, KeyboardKeyEvent},
};
use smithay::reexports::{
    wayland_server::{Display, DisplayHandle},
};
use smithay::utils::{Rectangle, Transform};

use wayland_server::{
    backend::{
        ClientId,
        ClientData,
        DisconnectReason
    },
    socket::ListeningSocket,
    protocol::wl_surface::{WlSurface},
};

use wayland_protocols::xdg_shell::server::xdg_toplevel::State;

struct Hutch {
    seat: Seat<Self>,
    seat_state: SeatState<Self>,

    shm_state: ShmState,
    xdg_shell_state: XdgShellState,
    compositor_state: CompositorState,
}

struct ClientState;

impl AsRef<ShmState> for Hutch {
    fn as_ref(&self) -> &ShmState {
        &self.shm_state
    }
}

impl BufferHandler for Hutch {
    fn buffer_destroyed(&mut self, buffer: &Buffer) {
        println!("Buffer detroyed: {:?}", buffer)
    }
}

impl SeatHandler<Self> for Hutch {
    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }
}

impl XdgShellHandler for Hutch {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn request(&mut self, dh: &mut DisplayHandle, request: XdgRequest) {
        dbg!(&request);

        match request {
            XdgRequest::NewToplevel { surface } => {
                surface.with_pending_state(
                    |state| {
                        state.states.set(State::Activated)
                    }
                );
                surface.send_configure(dh);
            },
            _ => {}
        }
    }
}

impl CompositorHandler for Hutch {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn commit(&mut self, dh: &mut DisplayHandle, surface: &WlSurface) {
        on_commit_buffer_handler(dh, surface);
    }
}

impl ClientData<Hutch> for ClientState {
    fn initialized(&self, client_id: ClientId) {
        println!("Initialized: {:?}", client_id)
    }

    fn disconnected(&self, client_id: ClientId, reason: DisconnectReason) {
        println!("Disconnected: {:?} with reason: {:?}", client_id, reason)
    }
}

fn log() -> slog::Logger {
    use slog::Drain;

    slog::Logger::root(slog_stdlog::StdLog.fuse(), slog::o!())
}

fn send_frames_surface_tree(
    dh: &mut DisplayHandle<'_>,
    surface: &WlSurface,
    time: u32,
) {
    with_surface_tree_downward(
        surface,
        (),
        |_, _, &()| TraversalAction::DoChildren(()),
        |_surf, states, &()| {
            for callback in states
                .cached_state
                .current::<SurfaceAttributes>()
                .frame_callbacks
                .drain(..)
            {
                callback.done(dh, time);
            }
        },
        |_, _, &()| true,
    );
}

fn run_winit() -> Result<(), Box<dyn Error>> {
    let log = log();

    let mut display: Display<Hutch> = Display::new()?;

    let seat_state: SeatState<Hutch> = SeatState::new();
    let seat: Seat<Hutch> = Seat::new(&mut display, "winit", None);

    let mut state: Hutch = {
        Hutch {
            seat,
            seat_state,
            shm_state: ShmState::new(&mut display, None),
            xdg_shell_state: XdgShellState::new(&mut display, None).0,
            compositor_state: CompositorState::new(&mut display, vec![], None),
        }
    };

    let listener: ListeningSocket = ListeningSocket::bind("wayland-3").unwrap();

    let mut clients= Vec::new();

    let (mut backend, mut winit) = winit::init(None)?;

    let start_time = std::time::Instant::now();

    let keyboard = state.seat
        .add_keyboard(
            &mut display.handle(),
            Default::default(),
            200,
            200,
            |_, _| {}
        )
        .unwrap();

    std::env::set_var("WAYLAND_DISPLAY", "wayland-3");
    std::process::Command::new("weston-terminal").spawn().ok();

    loop {
        winit.dispatch_new_events(
            |event| match event {
                WinitEvent::Resized { .. } => {}
                WinitEvent::Input(event) => match event {
                    InputEvent::Keyboard { event } => {
                        let dh = &mut display.handle();
                        keyboard.input::<(), _>(
                            dh,
                            event.key_code(),
                             event.state(),
                             0.into(),
                             0,
                             |_, _| {
                                 FilterResult::Forward
                            }
                        );
                    }
                    InputEvent::PointerMotionAbsolute { .. } => {
                        let dh = &mut display.handle();

                        state.xdg_shell_state.toplevel_surfaces(
                            |surfaces| {
                                for surface in surfaces {
                                    let surface = surface.wl_surface();

                                    keyboard.set_focus(dh, Some(surface), 0.into());

                                    break;
                                }
                            }
                        );
                    }
                    _ => {}
                },
                _ => (),
            }
        )?;

        backend.bind().unwrap();

        let size = backend.window_size().physical_size;
        let damage = Rectangle::from_loc_and_size((0, 0), size);

        backend
            .renderer()
            .render(
                size,
                Transform::Flipped180,
                |renderer,frame|  {
                    frame.clear([0.1, 0.0, 0.0, 1.0], &[damage.to_f64()]).unwrap();

                    state.xdg_shell_state.toplevel_surfaces(|surfaces| {
                        for surface in surfaces {
                            let dh = &mut display.handle();
                            let surface = surface.wl_surface();

                            draw_surface_tree(
                                renderer,
                                frame,
                                surface,
                                1.0,
                                (0, 0).into(),
                                &[damage.to_logical(1)],
                                dh,
                                &log,
                            )
                            .unwrap();

                            send_frames_surface_tree(
                                dh,
                                surface,
                                start_time
                                    .elapsed()
                                    .as_millis() as u32
                            );
                        }
                    }
                );
            }
        )?;

        if let Some(stream) = listener.accept()? {
            println!("Got a client: {:?}", stream);

            let client = display.insert_client(stream, Arc::new(ClientState)).unwerap();

            clients.push(client);
        }

        display.dispatch_clients(&mut state)?;
        display.flush_clients()?;

        backend.submit(Some(&[damage.to_logical(1)]), 1.0).unwrap();
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    run_winit();

    Ok(())
}

delegate_shm!(Hutch);
delegate_seat!(Hutch);
delegate_compositor!(Hutch);
delegate_xdg_shell!(Hutch);
