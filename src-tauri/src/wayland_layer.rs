use serde_json::{json, Value};
use std::env;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::time::{sleep, Duration};

struct WayfireClient {
    reader: tokio::sync::Mutex<tokio::io::ReadHalf<UnixStream>>,
    writer: tokio::sync::Mutex<tokio::io::WriteHalf<UnixStream>>,
}

impl WayfireClient {
    async fn connect() -> Result<Self, String> {
        let path = Self::find_socket().ok_or_else(|| {
            let debug = env::var("XDG_RUNTIME_DIR")
                .map(|d| format!("XDG_RUNTIME_DIR={d}"))
                .unwrap_or_default();
            format!("No Wayfire socket found ({debug})")
        })?;
        let stream = UnixStream::connect(&path)
            .await
            .map_err(|e| format!("Failed to connect to Wayfire socket at {path:?}: {e}"))?;
        let (reader, writer) = tokio::io::split(stream);
        Ok(Self {
            reader: tokio::sync::Mutex::new(reader),
            writer: tokio::sync::Mutex::new(writer),
        })
    }

    async fn call(&self, method: &str, data: Value) -> Result<Value, String> {
        let payload = json!({ "method": method, "data": data });
        let serialized =
            serde_json::to_vec(&payload).map_err(|e| format!("Serialize error: {e}"))?;
        let len = (serialized.len() as u32).to_le_bytes();

        {
            let mut writer = self.writer.lock().await;
            writer
                .write_all(&len)
                .await
                .map_err(|e| format!("Write error: {e}"))?;
            writer
                .write_all(&serialized)
                .await
                .map_err(|e| format!("Write error: {e}"))?;
            writer
                .flush()
                .await
                .map_err(|e| format!("Flush error: {e}"))?;
        }

        let mut header = [0u8; 4];
        self.reader
            .lock()
            .await
            .read_exact(&mut header)
            .await
            .map_err(|e| format!("Read header error: {e}"))?;
        let response_len = u32::from_le_bytes(header) as usize;
        let mut buffer = vec![0u8; response_len];
        self.reader
            .lock()
            .await
            .read_exact(&mut buffer)
            .await
            .map_err(|e| format!("Read body error: {e}"))?;

        let response: Value =
            serde_json::from_slice(&buffer).map_err(|e| format!("Parse error: {e}"))?;

        if let Some(error) = response.get("error").and_then(Value::as_str) {
            return Err(error.to_string());
        }

        Ok(response)
    }

    fn find_socket() -> Option<PathBuf> {
        for var in ["WAYFIRE_SOCKET", "WAYFIRE_IPC_SOCKET", "_WAYFIRE_SOCKET"] {
            if let Some(val) = env::var_os(var) {
                let path = PathBuf::from(val);
                if path.exists() {
                    return Some(path);
                }
            }
        }

        let runtime_dir = env::var_os("XDG_RUNTIME_DIR")?;
        let runtime_dir = PathBuf::from(&runtime_dir);

        if let Some(display) = env::var("WAYLAND_DISPLAY").ok() {
            let path = runtime_dir.join(format!("wayfire-{display}-.socket"));
            if path.exists() {
                return Some(path);
            }
        }

        for name in ["wayfire.socket", "wayfire-ipc.socket", "wayfire-ipc.sock"] {
            let path = runtime_dir.join(name);
            if path.exists() {
                return Some(path);
            }
        }

        if let Ok(entries) = std::fs::read_dir(&runtime_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("wayfire-") && name.ends_with(".socket") {
                        return Some(path);
                    }
                }
            }
        }

        None
    }
}

#[derive(serde::Deserialize, Debug)]
#[allow(dead_code)]
struct View {
    id: i64,
    title: Option<String>,
    #[serde(rename = "app-id")]
    app_id: Option<String>,
    mapped: Option<bool>,
    role: Option<String>,
    layer: Option<String>,
    #[serde(rename = "type")]
    type_field: Option<String>,
}

#[derive(serde::Deserialize, Debug)]
#[allow(dead_code)]
struct OutputGeometry {
    x: i64,
    y: i64,
    width: i64,
    height: i64,
}

#[derive(serde::Deserialize, Debug)]
struct Output {
    id: i64,
    geometry: OutputGeometry,
}

fn normalize(s: &str) -> String {
    s.to_lowercase().replace('-', " ").replace('_', " ")
}

fn view_matches(view: &View, lower_title: &str) -> bool {
    let view_title = normalize(&view.title.as_deref().unwrap_or_default());
    let app_id = normalize(&view.app_id.as_deref().unwrap_or_default());
    let role = normalize(&view.role.as_deref().unwrap_or_default());

    view_title == lower_title
        || view_title.contains(lower_title)
        || app_id == lower_title
        || app_id.contains(lower_title)
        || role == lower_title
        || role.contains(lower_title)
}

pub async fn configure_overlay_window(
    title: &str,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let client = WayfireClient::connect().await?;

    let lower_title = normalize(title);
    let mut view_id = None;
    let mut view_mapped = false;

    for attempt in 0..30 {
        let response = client.call("window-rules/list-views", Value::Null).await?;
        let views: Vec<View> =
            serde_json::from_value(response).map_err(|e| format!("Parse views error: {e}"))?;

        if let Some(view) = views.iter().find(|v| view_matches(v, &lower_title)) {
            view_id = Some(view.id as u64);
            view_mapped = view.mapped.unwrap_or(false);
            break;
        }

        if attempt >= 29 {
            let summary: Vec<String> = views
                .iter()
                .map(|v| {
                    format!(
                        "id={} title={:?} app_id={:?} role={:?} layer={:?}",
                        v.id, v.title, v.app_id, v.role, v.layer
                    )
                })
                .collect();
            return Err(format!(
                "Window '{title}' not found via Wayfire IPC after 30 attempts. Views: {summary:?}"
            ));
        }

        sleep(Duration::from_millis(100)).await;
    }

    let view_id = view_id.unwrap();

    let response = client.call("window-rules/list-outputs", Value::Null).await?;
    let outputs: Vec<Output> =
        serde_json::from_value(response).map_err(|e| format!("Parse outputs error: {e}"))?;

    let output_id = outputs
        .iter()
        .find(|o| {
            let g = &o.geometry;
            (x as i64) >= g.x && (x as i64) < g.x + g.width
        })
        .map(|o| o.id as u64)
        .or_else(|| outputs.first().map(|o| o.id as u64));

    let geometry = json!({
        "id": view_id,
        "geometry": {
            "x": x as i64,
            "y": y as i64,
            "width": width as i64,
            "height": height as i64,
        }
    });

    let mut geometry_data = geometry;
    if let Some(oid) = output_id {
        geometry_data["output_id"] = json!(oid);
    }

    client
        .call("window-rules/configure-view", geometry_data)
        .await?;

    if !view_mapped {
        for _ in 0..30 {
            sleep(Duration::from_millis(100)).await;
            let Ok(response) = client.call("window-rules/list-views", Value::Null).await else {
                continue;
            };
            let Ok(all_views) = serde_json::from_value::<Vec<View>>(response) else {
                continue;
            };
            let Some(mapped_view) = all_views.into_iter().find(|v| v.id == view_id as i64) else {
                continue;
            };
            if mapped_view.mapped != Some(true) {
                continue;
            }
            let _ = client.call("window-rules/configure-view", json!({
                "id": view_id,
                "geometry": { "x": x as i64, "y": y as i64, "width": width as i64, "height": height as i64 },
            })).await;
            break;
        }
    }

    client
        .call(
            "wm-actions/set-always-on-top",
            json!({ "view_id": view_id, "state": true }),
        )
        .await?;

    client
        .call(
            "wm-actions/set-sticky",
            json!({ "view_id": view_id, "state": true }),
        )
        .await?;

    Ok(())
}
