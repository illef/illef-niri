use niri_ipc::socket::Socket;
use niri_ipc::{Action, Request, Response, Window};

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

pub fn sticked_window_ids_in_active_workspace(
    socket: &mut Socket,
) -> Result<Vec<u64>, &'static str> {
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
    let windows_ids = match socket.send(Request::Windows) {
        Ok(Ok(Response::Windows(windows))) => windows
            .iter()
            .filter(|w| w.workspace_id == Some(focused_workspace_id) && !w.is_floating)
            .map(|w| w.id)
            .collect(),
        _ => Vec::new(),
    };
    Ok(windows_ids)
}

pub fn set_centered_window_if_only_one(socket: &mut Socket) -> Result<Option<u64>, &'static str> {
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
            .filter(|w| w.workspace_id == Some(focused_workspace_id) && !w.is_floating)
            .map(|w| w.id)
            .collect(),
        _ => Vec::new(),
    };
    if let [id] = windows_ids_in_workspace.as_slice() {
        let _ = socket.send(Request::Action(Action::CenterWindow { id: Some(*id) }));
        Ok(Some(*id))
    } else {
        Ok(None)
    }
}
