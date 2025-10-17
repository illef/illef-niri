use niri_ipc::socket::Socket;
use niri_ipc::{Action, Request, Response, Window};

pub fn get_niri_socket_path() -> String {
    let xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());

    loop {
        let file = std::fs::read_dir(&xdg_runtime_dir)
            .unwrap()
            .filter_map(|file| file.ok())
            .filter_map(|file| match file.path().to_str() {
                Some(path) if path.contains("niri") => Some(path.to_string()),
                _ => None,
            })
            .next();

        if file.is_some() {
            return file.unwrap();
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

pub fn get_master_slave_windows(windows: &[Window]) -> (Option<&Window>, Option<&Window>) {
    let windows_in_active_workspaces = windows
        .iter()
        .find(|w| w.is_focused)
        .and_then(|w| w.workspace_id)
        .map(|workspace_id| {
            windows
                .iter()
                .filter(|w| w.workspace_id == Some(workspace_id))
                .collect::<Vec<_>>()
        });

    if let Some(windows_in_active_workspaces) = windows_in_active_workspaces {
        let master_window = windows_in_active_workspaces
            .iter()
            .find(|w| {
                w.layout
                    .pos_in_scrolling_layout
                    .is_some_and(|(column, _)| column == 1)
            })
            .copied();

        let slave_window = windows_in_active_workspaces
            .iter()
            .find(|w| {
                w.layout
                    .pos_in_scrolling_layout
                    .is_some_and(|(column, _)| column == 2)
            })
            .copied();

        (master_window, slave_window)
    } else {
        (None, None)
    }
}

pub fn set_window_width(
    socket: &mut Socket,
    window_id: u64,
    proportion: f64,
) -> Result<(), &'static str> {
    let _ = socket
        .send(Request::Action(Action::SetWindowWidth {
            id: Some(window_id),
            change: niri_ipc::SizeChange::SetProportion(proportion * 100.0),
        }))
        .map_err(|_| "failed to set window width")?;

    Ok(())
}

pub fn set_centered_window_if_only_one(socket: &mut Socket) -> Result<(), &'static str> {
    let focused_workspace_id = socket
        .send(Request::Workspaces)
        .map_err(|_| "failed to get workspaces")
        .and_then(|resp| match resp {
            Ok(Response::Workspaces(workspaces)) => workspaces
                .iter()
                .find(|w| w.is_focused)
                .map(|w| w.id)
                .ok_or("no focused workspace"),
            _ => Err("unexpected response"),
        })?;
    let windows_ids_in_workspace = match socket.send(Request::Windows) {
        Ok(Ok(Response::Windows(windows))) => windows
            .iter()
            .filter(|w| w.workspace_id == Some(focused_workspace_id))
            .map(|w| w.id)
            .collect(),
        _ => Vec::new(),
    };
    if windows_ids_in_workspace.len() == 1 {
        let _ = socket.send(Request::Action(Action::CenterWindow {
            id: Some(windows_ids_in_workspace[0]),
        }));
    }

    Ok(())
}
