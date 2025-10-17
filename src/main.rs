use axum::{Router, routing::post};
use niri_ipc::socket::Socket;
use niri_ipc::{Action, Event, Request, Response};
use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::process::exit;
use tokio::task;

mod niri;

async fn change_layout() -> Result<&'static str, &'static str> {
    let mut socket = Socket::connect().map_err(|_| "failed to connect to niri")?;

    let response = socket
        .send(Request::Windows)
        .map_err(|_| "failed to get windows")?
        .map_err(|_| "failed to get windows")?;

    let Response::Windows(windows) = response else {
        return Err("unexpected response from niri");
    };

    let (master, slave) = match niri::get_master_slave_windows(&windows) {
        (Some(master), Some(slave)) => (master, slave),
        _ => return Err("not enough windows to change layout"),
    };

    let (master_prop, slave_prop) = if master.layout.window_size.0 == slave.layout.window_size.0 {
        (0.66667, 0.33333)
    } else {
        (0.5, 0.5)
    };

    niri::set_window_width(&mut socket, master.id, master_prop)?;
    niri::set_window_width(&mut socket, slave.id, slave_prop)?;

    return Ok("go master/slave mode");
}

fn spawn_niri_event_handler() {
    task::spawn_blocking(|| -> std::io::Result<()> {
        let mut socket = Socket::connect()?;
        let mut action_socket = Socket::connect()?;

        let reply = socket.send(Request::EventStream)?;
        if !matches!(reply, Ok(Response::Handled)) {
            return Ok(());
        }

        let mut read_event = socket.read_events();
        let mut opened_windows_ids: BTreeSet<u64> = BTreeSet::new();

        while let Ok(event) = read_event() {
            match event {
                Event::WindowLayoutsChanged { .. } => {
                    if let Ok(windows_ids) =
                        niri::sticked_window_ids_in_active_workspace(&mut action_socket)
                    {
                        if let [window_id] = windows_ids.as_slice() {
                            let _ = action_socket.send(Request::Action(Action::CenterWindow {
                                id: Some(*window_id),
                            }));
                        }
                    }
                }
                Event::WindowClosed { id } => {
                    opened_windows_ids.remove(&id);
                    if let Ok(windows_ids) =
                        niri::sticked_window_ids_in_active_workspace(&mut action_socket)
                    {
                        match windows_ids.as_slice() {
                            [window_id] => {
                                let _ = action_socket.send(Request::Action(Action::CenterWindow {
                                    id: Some(*window_id),
                                }));
                                let _ =
                                    niri::set_window_width(&mut action_socket, *window_id, 0.66667);
                            }
                            _ => {}
                        }
                    }
                }
                Event::WindowOpenedOrChanged { window } => {
                    if opened_windows_ids.contains(&window.id) {
                        continue;
                    } else {
                        opened_windows_ids.insert(window.id);
                    }
                    if matches!(
                        window.app_id.as_deref(),
                        Some("Logseq") | Some("illef.illpad")
                    ) {
                        continue;
                    }
                    if window.is_floating {
                        continue;
                    }
                    if let Ok(windows_ids) =
                        niri::sticked_window_ids_in_active_workspace(&mut action_socket)
                    {
                        match windows_ids.as_slice() {
                            [window_id] => {
                                let _ = action_socket.send(Request::Action(Action::CenterWindow {
                                    id: Some(*window_id),
                                }));
                                let _ =
                                    niri::set_window_width(&mut action_socket, *window_id, 0.66667);
                            }
                            [one, two] => {
                                let _ = niri::set_window_width(&mut action_socket, *one, 0.5);
                                let _ = niri::set_window_width(&mut action_socket, *two, 0.5);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        exit(1);
    });
}

async fn run_http_server() -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new().route("/layout/change", post(change_layout));

    let addr = SocketAddr::from(([127, 0, 0, 1], 9999));
    println!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    spawn_niri_event_handler();
    run_http_server().await?;
    Ok(())
}
