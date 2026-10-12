//! OpenDeck action for the DesignCraft command line interface.
//!
//! Arguments are stored as a JSON string array and sent directly to the CLI.
//! Deliberately never invoking a shell preserves paths, JSON parameters, and user input exactly.

use std::{path::PathBuf, process::Stdio, time::Duration};

use openaction::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};
use tokio::{net::TcpStream, process::Command};

const DEFAULT_EXECUTABLE: &str = "designcraft-cli";
const DEFAULT_DESKTOP_EXECUTABLE: &str = "designcraft";
const DEFAULT_CONTROL_PORT: u16 = 7979;
const CATALOG_ARGUMENTS: &[&str] = &["commands"];
const DEFAULT_ARGUMENTS: &str = r#"["commands"]"#;
const MAX_ARGUMENTS: usize = 128;
const MAX_ARGUMENT_BYTES: usize = 16 * 1024;
const MAX_CATALOG_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct CommandSettings {
    /// CLI executable or absolute path. Empty uses the app's conventional CLI name.
    executable: String,
    /// JSON string array, e.g. ["exec", "timeline.play", "{}"].
    arguments: String,
    /// Optional directory in which to run the command.
    working_directory: String,
    /// A user-visible title shown before the optional command output.
    title: String,
    /// Keep stdout/stderr (trimmed) on the Stream Deck key after a successful run.
    show_output: bool,
    /// Wall-clock limit for a process. Zero uses 30 seconds; maximum is 300 seconds.
    timeout_seconds: u64,
}

impl Default for CommandSettings {
    fn default() -> Self {
        Self {
            executable: String::new(),
            arguments: DEFAULT_ARGUMENTS.to_string(),
            working_directory: String::new(),
            title: String::new(),
            show_output: false,
            timeout_seconds: 30,
        }
    }
}

impl CommandSettings {
    fn executable(&self) -> &str {
        let executable = self.executable.trim();
        if executable.is_empty() {
            DEFAULT_EXECUTABLE
        } else {
            executable
        }
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(if self.timeout_seconds == 0 {
            30
        } else {
            self.timeout_seconds.clamp(1, 300)
        })
    }

    fn title(&self) -> Option<String> {
        (!self.title.trim().is_empty()).then(|| self.title.trim().to_string())
    }
}

#[derive(Debug)]
enum ExecutionError {
    Configuration(String),
    Process(String),
    TimedOut(u64),
    Failed { code: Option<i32>, output: String },
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Configuration(message) | Self::Process(message) => f.write_str(message),
            Self::TimedOut(seconds) => write!(f, "Timed out after {seconds} seconds"),
            Self::Failed { code, output } => {
                write!(
                    f,
                    "DesignCraft CLI exited with {}",
                    code.map_or_else(|| "a signal".into(), |code| format!("code {code}"))
                )?;
                if !output.is_empty() {
                    write!(f, ": {output}")?;
                }
                Ok(())
            }
        }
    }
}

fn parse_arguments(raw: &str) -> Result<Vec<String>, ExecutionError> {
    let parsed: Value = serde_json::from_str(raw.trim()).map_err(|error| {
        ExecutionError::Configuration(format!(
            "Arguments must be a JSON array of strings: {error}"
        ))
    })?;
    let values = parsed.as_array().ok_or_else(|| {
        ExecutionError::Configuration("Arguments must be a JSON array of strings.".into())
    })?;
    if values.is_empty() {
        return Err(ExecutionError::Configuration(
            "Add at least one CLI argument, for example [\"commands\", \"--json\"].".into(),
        ));
    }
    if values.len() > MAX_ARGUMENTS {
        return Err(ExecutionError::Configuration(format!(
            "At most {MAX_ARGUMENTS} arguments are allowed."
        )));
    }
    values
        .iter()
        .map(|value| {
            let value = value.as_str().ok_or_else(|| {
                ExecutionError::Configuration(
                    "Every argument must be a string; numbers and JSON values must be quoted."
                        .into(),
                )
            })?;
            if value.len() > MAX_ARGUMENT_BYTES {
                return Err(ExecutionError::Configuration(format!(
                    "An argument is longer than {MAX_ARGUMENT_BYTES} bytes."
                )));
            }
            Ok(value.to_string())
        })
        .collect()
}

fn summary(stdout: &[u8], stderr: &[u8]) -> String {
    let output = if !stderr.is_empty() { stderr } else { stdout };
    let text = String::from_utf8_lossy(output).replace(['\n', '\r'], " ");
    let text = text.trim();
    let mut end = text.len().min(120);
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    let suffix = if text.len() > end { "…" } else { "" };
    format!("{}{}", &text[..end], suffix)
}

async fn execute(
    settings: &CommandSettings,
    arguments: Vec<String>,
) -> Result<String, ExecutionError> {
    let mut command = Command::new(settings.executable());
    command
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if !settings.working_directory.trim().is_empty() {
        command.current_dir(PathBuf::from(settings.working_directory.trim()));
    }
    let seconds = settings.timeout().as_secs();
    let output = tokio::time::timeout(settings.timeout(), command.output())
        .await
        .map_err(|_| ExecutionError::TimedOut(seconds))?
        .map_err(|error| {
            ExecutionError::Process(format!(
                "Could not start {}: {error}",
                settings.executable()
            ))
        })?;
    let output_summary = summary(&output.stdout, &output.stderr);
    if output.status.success() {
        Ok(output_summary)
    } else {
        Err(ExecutionError::Failed {
            code: output.status.code(),
            output: output_summary,
        })
    }
}

async fn report(
    instance: &Instance,
    settings: &CommandSettings,
    result: Result<String, ExecutionError>,
    to_inspector: bool,
) {
    match result {
        Ok(output) => {
            let title = if settings.show_output && !output.is_empty() {
                output.clone()
            } else {
                settings.title().unwrap_or_else(|| "Done".to_string())
            };
            let _ = instance.set_title(Some(title), None).await;
            let _ = instance.show_ok().await;
            if to_inspector {
                let _ = instance
                    .send_to_property_inspector(
                        json!({"kind":"result", "ok":true, "message": output}),
                    )
                    .await;
            }
        }
        Err(error) => {
            let message = error.to_string();
            log::warn!("DesignCraft CLI action failed: {message}");
            let _ = instance.set_title(Some("Error".to_string()), None).await;
            let _ = instance.show_alert().await;
            if to_inspector {
                let _ = instance
                    .send_to_property_inspector(
                        json!({"kind":"result", "ok":false, "message": message}),
                    )
                    .await;
            }
        }
    }
}

async fn run_action(instance: &Instance, settings: &CommandSettings, to_inspector: bool) {
    let result = parse_arguments(&settings.arguments).and_then(|arguments| {
        // Validation above is synchronous; the process work stays on the async runtime.
        Ok(arguments)
    });
    let result = match result {
        Ok(arguments) => execute(settings, arguments).await,
        Err(error) => Err(error),
    };
    report(instance, settings, result, to_inspector).await;
}

/// Execute focused actions assembled from documented DesignCraft control-channel commands.
/// The original action remains available when an advanced user needs the raw CLI.
async fn run_arguments(
    instance: &Instance,
    settings: &CommandSettings,
    arguments: Result<Vec<String>, ExecutionError>,
) {
    let result = match arguments {
        Ok(arguments) => execute(settings, arguments).await,
        Err(error) => Err(error),
    };
    report(instance, settings, result, false).await;
}

fn required(value: &str, name: &str) -> Result<String, ExecutionError> {
    let value = value.trim();
    (!value.is_empty())
        .then(|| value.to_string())
        .ok_or_else(|| ExecutionError::Configuration(format!("{name} is required.")))
}

fn parse_rect(value: &str) -> Result<Value, ExecutionError> {
    if value.trim().is_empty() {
        return Ok(json!([72, 72, 400, 200]));
    }
    let rect: Value = serde_json::from_str(value).map_err(|_| {
        ExecutionError::Configuration(
            "Frame rectangle must be JSON such as [72,72,400,200].".into(),
        )
    })?;
    let valid = rect
        .as_array()
        .is_some_and(|values| values.len() == 4 && values.iter().all(Value::is_number));
    valid.then_some(rect).ok_or_else(|| {
        ExecutionError::Configuration(
            "Frame rectangle needs four numeric coordinates [x0,y0,x1,y1].".into(),
        )
    })
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct DesignWorkflowSettings {
    executable: String,
    port: String,
    operation: String,
    document_path: String,
    output_path: String,
    text: String,
    rectangle: String,
    pages: u16,
    layer_name: String,
    title: String,
    show_output: bool,
    timeout_seconds: u64,
}

impl Default for DesignWorkflowSettings {
    fn default() -> Self {
        Self {
            executable: String::new(),
            port: "7979".to_string(),
            operation: "new_a4".to_string(),
            document_path: String::new(),
            output_path: String::new(),
            text: String::new(),
            rectangle: "[72,72,400,200]".to_string(),
            pages: 1,
            layer_name: "Artwork".to_string(),
            title: String::new(),
            show_output: false,
            timeout_seconds: 30,
        }
    }
}

impl DesignWorkflowSettings {
    fn command_settings(&self) -> CommandSettings {
        CommandSettings {
            executable: self.executable.clone(),
            arguments: DEFAULT_ARGUMENTS.to_string(),
            working_directory: String::new(),
            title: self.title.clone(),
            show_output: self.show_output,
            timeout_seconds: self.timeout_seconds,
        }
    }

    fn app_command(&self, command: &str, params: Value) -> Result<Vec<String>, ExecutionError> {
        let port = required(&self.port, "DesignCraft control port")?;
        Ok(vec![
            "app".to_string(),
            "--port".to_string(),
            port,
            command.to_string(),
            serde_json::to_string(&params)
                .map_err(|error| ExecutionError::Configuration(error.to_string()))?,
        ])
    }

    fn arguments(&self) -> Result<Vec<String>, ExecutionError> {
        let rect = || parse_rect(&self.rectangle);
        let pages = self.pages.clamp(1, 9999);
        match self.operation.as_str() {
            "new_a4" => self.app_command("file.new", json!({"preset":"A4","pages":pages,"title":self.title.trim()})),
            "open" => self.app_command("file.open", json!({"path":required(&self.document_path, "Document path")?})),
            "text_frame" => self.app_command("frame.create", json!({"rect":rect()?,"content":"text","text":self.text,"caret":false})),
            "graphic_frame" => self.app_command("frame.create", json!({"rect":rect()?,"content":"graphic"})),
            "layer" => self.app_command("layer.new", json!({"name":required(&self.layer_name, "Layer name")?})),
            "duplicate" => self.app_command("edit.duplicate", json!({})),
            "group" => self.app_command("object.group", json!({})),
            "ungroup" => self.app_command("object.ungroup", json!({})),
            "align_left" => self.app_command("object.align", json!({"edge":"left","to":"margins"})),
            "align_center" => self.app_command("object.align", json!({"edge":"hcenter","to":"page"})),
            "align_top" => self.app_command("object.align", json!({"edge":"top","to":"margins"})),
            "save" => self.app_command("file.save", json!({})),
            "export_pdf" => self.app_command("file.exportPdf", json!({"path":required(&self.output_path, "PDF output path")?,"tagged":true,"standard":"x4"})),
            _ => Err(ExecutionError::Configuration("Choose a DesignCraft workflow operation.".into())),
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct DesignRotarySettings {
    executable: String,
    port: String,
    mode: String,
    step: u16,
    press: String,
    title: String,
    timeout_seconds: u64,
}

impl Default for DesignRotarySettings {
    fn default() -> Self {
        Self {
            executable: String::new(),
            port: "7979".to_string(),
            mode: "zoom".to_string(),
            step: 6,
            press: "fit_page".to_string(),
            title: "Layout Zoom".to_string(),
            timeout_seconds: 30,
        }
    }
}

impl DesignRotarySettings {
    fn command_settings(&self) -> CommandSettings {
        CommandSettings {
            executable: self.executable.clone(),
            arguments: DEFAULT_ARGUMENTS.to_string(),
            working_directory: String::new(),
            title: self.title.clone(),
            show_output: false,
            timeout_seconds: self.timeout_seconds,
        }
    }

    fn port(&self) -> Result<String, ExecutionError> {
        required(&self.port, "DesignCraft control port")
    }

    fn app_command(&self, command: &str, params: Value) -> Result<Vec<String>, ExecutionError> {
        Ok(vec![
            "app".to_string(),
            "--port".to_string(),
            self.port()?,
            command.to_string(),
            serde_json::to_string(&params)
                .map_err(|error| ExecutionError::Configuration(error.to_string()))?,
        ])
    }

    fn method(&self, method: &str, params: Value) -> Result<Vec<String>, ExecutionError> {
        Ok(vec![
            "app".to_string(),
            "--port".to_string(),
            self.port()?,
            "--method".to_string(),
            method.to_string(),
            serde_json::to_string(&params)
                .map_err(|error| ExecutionError::Configuration(error.to_string()))?,
        ])
    }

    fn tick_arguments(&self, ticks: i16) -> Result<Vec<String>, ExecutionError> {
        let step = f64::from(self.step.max(1));
        match self.mode.as_str() {
            // DesignCraft's UI command is used here, rather than a fake app-wide zoom, so it
            // retains the canvas anchor and all native zoom behaviour.
            "zoom" => self.method(
                "ui.menu.invoke",
                json!({"command":if ticks >= 0 {"view.zoomIn"} else {"view.zoomOut"},"params":{}}),
            ),
            "move_horizontal" => self.app_command(
                "transform.move",
                json!({"dx":f64::from(ticks) * step,"dy":0.0}),
            ),
            "move_vertical" => self.app_command(
                "transform.move",
                json!({"dx":0.0,"dy":f64::from(ticks) * step}),
            ),
            "rotate" => {
                self.app_command("transform.rotate", json!({"angle":f64::from(ticks) * step}))
            }
            _ => Err(ExecutionError::Configuration(
                "Rotary mode must be zoom, move_horizontal, move_vertical, or rotate.".into(),
            )),
        }
    }

    fn press_arguments(&self) -> Result<Vec<String>, ExecutionError> {
        match self.press.as_str() {
            "fit_page" => self.method("ui.set", json!({"fit":"page"})),
            "fit_spread" => self.method("ui.set", json!({"fit":"spread"})),
            "save" => self.app_command("file.save", json!({})),
            "none" => Ok(Vec::new()),
            _ => Err(ExecutionError::Configuration(
                "Choose a rotary press action.".into(),
            )),
        }
    }
}

fn catalog_items(stdout: &str) -> Vec<Value> {
    let Ok(value) = serde_json::from_str::<Value>(stdout) else {
        return Vec::new();
    };
    let items = value
        .as_array()
        .cloned()
        .or_else(|| value.get("commands").and_then(Value::as_array).cloned())
        .unwrap_or_default();
    items
        .into_iter()
        .filter_map(|item| {
            if let Some(id) = item.as_str() {
                return Some(json!({"id": id, "label": id}));
            }
            let id = ["id", "command", "tool", "name"]
                .into_iter()
                .find_map(|key| item.get(key).and_then(Value::as_str))?;
            let label = item
                .get("label")
                .or_else(|| item.get("name"))
                .and_then(Value::as_str)
                .unwrap_or(id);
            Some(json!({"id": id, "label": label}))
        })
        .collect()
}

async fn send_catalog(instance: &Instance, settings: &CommandSettings) {
    let mut command = Command::new(settings.executable());
    command
        .args(CATALOG_ARGUMENTS)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if !settings.working_directory.trim().is_empty() {
        command.current_dir(PathBuf::from(settings.working_directory.trim()));
    }
    let result = tokio::time::timeout(Duration::from_secs(15), command.output()).await;
    let reply = match result {
        Ok(Ok(output)) if output.status.success() && output.stdout.len() <= MAX_CATALOG_BYTES => {
            let text = String::from_utf8_lossy(&output.stdout);
            let items = catalog_items(&text);
            if items.is_empty() {
                json!({"kind":"catalog", "ok":false, "message":"The installed CLI did not return a JSON command catalog."})
            } else {
                json!({"kind":"catalog", "ok":true, "items":items})
            }
        }
        Ok(Ok(output)) => {
            json!({"kind":"catalog", "ok":false, "message":format!("Could not read the command catalog: {}", summary(&output.stdout, &output.stderr))})
        }
        Ok(Err(error)) => {
            json!({"kind":"catalog", "ok":false, "message":format!("Could not start {}: {error}", settings.executable())})
        }
        Err(_) => {
            json!({"kind":"catalog", "ok":false, "message":"Timed out while loading the command catalog."})
        }
    };
    let _ = instance.send_to_property_inspector(reply).await;
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct LaunchSettings {
    executable: String,
    control_host: String,
    control_port: u16,
    working_directory: String,
    title: String,
}

impl Default for LaunchSettings {
    fn default() -> Self {
        Self {
            executable: String::new(),
            control_host: "127.0.0.1".into(),
            control_port: DEFAULT_CONTROL_PORT,
            working_directory: String::new(),
            title: "Open DesignCraft".into(),
        }
    }
}

impl LaunchSettings {
    fn executable(&self) -> &str {
        let value = self.executable.trim();
        if value.is_empty() {
            DEFAULT_DESKTOP_EXECUTABLE
        } else {
            value
        }
    }
    fn host(&self) -> &str {
        let value = self.control_host.trim();
        if value.is_empty() {
            "127.0.0.1"
        } else {
            value
        }
    }
    fn port(&self) -> Result<u16, ExecutionError> {
        if self.control_port == 0 {
            Err(ExecutionError::Configuration(
                "Choose a control port from 1 to 65535.".into(),
            ))
        } else {
            Ok(self.control_port)
        }
    }
    fn arguments(&self) -> Result<Vec<String>, ExecutionError> {
        Ok(vec!["--control".into(), self.port()?.to_string()])
    }
}

fn loopback_host(host: &str) -> bool {
    matches!(
        host.to_ascii_lowercase().as_str(),
        "127.0.0.1" | "::1" | "localhost"
    )
}

async fn launch_designcraft(settings: &LaunchSettings) -> Result<String, ExecutionError> {
    let host = settings.host();
    if !loopback_host(host) {
        return Err(ExecutionError::Configuration("DesignCraft exposes its control channel only on localhost. Use 127.0.0.1, localhost, or ::1.".into()));
    }
    let port = settings.port()?;
    if matches!(
        tokio::time::timeout(Duration::from_millis(250), TcpStream::connect((host, port))).await,
        Ok(Ok(_))
    ) {
        return Ok(format!("DesignCraft is already ready at {host}:{port}."));
    }
    let mut command = Command::new(settings.executable());
    command
        .args(settings.arguments()?)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(false);
    if !settings.working_directory.trim().is_empty() {
        command.current_dir(PathBuf::from(settings.working_directory.trim()));
    }
    command.spawn().map_err(|error| {
        ExecutionError::Process(format!("Could not open {}: {error}", settings.executable()))
    })?;
    Ok(format!("Starting DesignCraft at {host}:{port}."))
}

async fn report_launch(
    instance: &Instance,
    settings: &LaunchSettings,
    result: Result<String, ExecutionError>,
    inspector: bool,
) {
    match result {
        Ok(message) => {
            let _ = instance.set_title(Some(settings.title.clone()), None).await;
            let _ = instance.show_ok().await;
            if inspector {
                let _ = instance
                    .send_to_property_inspector(
                        json!({"kind":"launch","ok":true,"message":message}),
                    )
                    .await;
            }
        }
        Err(error) => {
            let message = error.to_string();
            log::warn!("DesignCraft launcher failed: {message}");
            let _ = instance.set_title(Some("Error".to_string()), None).await;
            let _ = instance.show_alert().await;
            if inspector {
                let _ = instance
                    .send_to_property_inspector(
                        json!({"kind":"launch","ok":false,"message":message}),
                    )
                    .await;
            }
        }
    }
}

struct LaunchDesignCraft;
#[async_trait]
impl Action for LaunchDesignCraft {
    const UUID: &'static str = "wt.luizgustavo.getcraft.designcraft.launch";
    type Settings = LaunchSettings;
    async fn key_down(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        report_launch(
            instance,
            settings,
            launch_designcraft(settings).await,
            false,
        )
        .await;
        Ok(())
    }
    async fn will_appear(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        let _ = instance.set_title(Some(settings.title.clone()), None).await;
        Ok(())
    }
    async fn send_to_plugin(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
        payload: &Value,
    ) -> OpenActionResult<()> {
        if payload.get("request").and_then(Value::as_str) == Some("launch") {
            let effective = payload
                .get("settings")
                .and_then(|value| serde_json::from_value::<LaunchSettings>(value.clone()).ok())
                .unwrap_or_else(|| settings.clone());
            report_launch(
                instance,
                &effective,
                launch_designcraft(&effective).await,
                true,
            )
            .await;
        }
        Ok(())
    }
}

struct RunDesignCraftCli;

#[async_trait]
impl Action for RunDesignCraftCli {
    const UUID: &'static str = "wt.luizgustavo.getcraft.designcraft.run";
    type Settings = CommandSettings;

    async fn key_down(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        run_action(instance, settings, false).await;
        Ok(())
    }

    async fn dial_down(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.key_down(instance, settings).await
    }

    async fn will_appear(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        if let Some(title) = settings.title() {
            let _ = instance.set_title(Some(title), None).await;
        }
        Ok(())
    }

    async fn send_to_plugin(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
        payload: &Value,
    ) -> OpenActionResult<()> {
        let effective = payload
            .get("settings")
            .and_then(|value| serde_json::from_value::<CommandSettings>(value.clone()).ok())
            .unwrap_or_else(|| settings.clone());
        match payload.get("request").and_then(Value::as_str) {
            Some("catalog") => send_catalog(instance, &effective).await,
            Some("test") => run_action(instance, &effective, true).await,
            _ => {}
        }
        Ok(())
    }
}

struct DesignWorkflow;

#[async_trait]
impl Action for DesignWorkflow {
    const UUID: &'static str = "wt.luizgustavo.getcraft.designcraft.workflow";
    type Settings = DesignWorkflowSettings;

    async fn key_down(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        let command = settings.command_settings();
        run_arguments(instance, &command, settings.arguments()).await;
        Ok(())
    }

    async fn dial_down(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.key_down(instance, settings).await
    }

    async fn will_appear(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        let title = if settings.title.trim().is_empty() {
            "Layout Tool"
        } else {
            settings.title.trim()
        };
        let _ = instance.set_title(Some(title.to_string()), None).await;
        Ok(())
    }
}

struct DesignRotary;

#[async_trait]
impl Action for DesignRotary {
    const UUID: &'static str = "wt.luizgustavo.getcraft.designcraft.rotary";
    type Settings = DesignRotarySettings;

    async fn dial_rotate(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
        ticks: i16,
        _pressed: bool,
    ) -> OpenActionResult<()> {
        if ticks == 0 {
            return Ok(());
        }
        let count = if settings.mode == "zoom" {
            usize::from(ticks.unsigned_abs()).min(32)
        } else {
            1
        };
        let command = settings.command_settings();
        for _ in 0..count {
            run_arguments(instance, &command, settings.tick_arguments(ticks)).await;
        }
        Ok(())
    }

    async fn dial_down(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        let args = settings.press_arguments();
        if matches!(args, Ok(ref values) if values.is_empty()) {
            return Ok(());
        }
        let command = settings.command_settings();
        run_arguments(instance, &command, args).await;
        Ok(())
    }

    async fn key_down(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.dial_down(instance, settings).await
    }

    async fn will_appear(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        let _ = instance.set_title(Some(settings.title.clone()), None).await;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> OpenActionResult<()> {
    let _ = TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Stdout,
        ColorChoice::Never,
    );
    register_action(LaunchDesignCraft).await;
    register_action(RunDesignCraftCli).await;
    register_action(DesignWorkflow).await;
    register_action(DesignRotary).await;
    run(std::env::args().collect()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launcher_uses_the_configured_control_port() {
        let settings = LaunchSettings {
            control_port: 24567,
            ..Default::default()
        };
        assert_eq!(settings.arguments().unwrap(), ["--control", "24567"]);
    }

    #[test]
    fn accepts_a_string_argument_vector() {
        assert_eq!(
            parse_arguments(r#"["exec","timeline.play","{}"]"#).unwrap(),
            ["exec", "timeline.play", "{}"]
        );
    }

    #[test]
    fn rejects_non_string_arguments() {
        assert!(parse_arguments(r#"["exec", 4]"#).is_err());
    }

    #[test]
    fn stderr_is_preferred_in_a_failure_summary() {
        assert_eq!(summary(b"normal", b"bad\nnews"), "bad news");
    }

    #[test]
    fn has_a_usable_default_command() {
        assert_eq!(
            parse_arguments(&CommandSettings::default().arguments).unwrap(),
            CATALOG_ARGUMENTS
                .iter()
                .map(|value| (*value).to_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn honors_a_one_second_timeout() {
        assert_eq!(
            CommandSettings {
                timeout_seconds: 1,
                ..Default::default()
            }
            .timeout(),
            Duration::from_secs(1)
        );
    }

    #[test]
    fn creates_a_typed_frame_through_the_running_app() {
        let settings = DesignWorkflowSettings {
            operation: "text_frame".into(),
            text: "Headline".into(),
            ..Default::default()
        };
        assert_eq!(
            settings.arguments().unwrap(),
            [
                "app",
                "--port",
                "7979",
                "frame.create",
                r#"{"caret":false,"content":"text","rect":[72,72,400,200],"text":"Headline"}"#
            ]
        );
    }

    #[test]
    fn export_requires_an_explicit_output_path() {
        let settings = DesignWorkflowSettings {
            operation: "export_pdf".into(),
            ..Default::default()
        };
        assert!(settings.arguments().is_err());
    }

    #[test]
    fn rotary_moves_selection_using_engine_units() {
        let settings = DesignRotarySettings {
            mode: "move_horizontal".into(),
            step: 5,
            ..Default::default()
        };
        assert_eq!(
            settings.tick_arguments(-2).unwrap(),
            [
                "app",
                "--port",
                "7979",
                "transform.move",
                r#"{"dx":-10.0,"dy":0.0}"#
            ]
        );
    }
}
