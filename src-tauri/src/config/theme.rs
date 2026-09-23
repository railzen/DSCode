use std::fs;
use std::sync::{Mutex, OnceLock};
#[cfg(target_os = "macos")]
use tauri::Manager;
use tauri::{AppHandle, Emitter};

use super::runtime::get_dsh_data_path;

/// dsh 主题偏好
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DshTheme {
    Dark,
    Light,
    System,
}

const DEFAULT_THEME: DshTheme = DshTheme::Light;

static LAST_EMITTED: OnceLock<Mutex<Option<DshTheme>>> = OnceLock::new();

/// 读取 dsh 主题偏好；优先读取新版 web 档案配置，兼容旧版 settings.yaml
pub fn get_dsh_theme(app_handle: &AppHandle) -> DshTheme {
    let home = get_dsh_data_path(app_handle);
    let patch_path = home.join("profiles/web/cordis.patch.yml");
    if let Ok(content) = fs::read_to_string(&patch_path) {
        if let Some(theme) = parse_patch_theme_preference(&content) {
            return theme;
        }
    }
    let settings_path = home.join("settings.yaml");
    fs::read_to_string(&settings_path)
        .ok()
        .and_then(|content| parse_theme_preference(&content))
        .unwrap_or(DEFAULT_THEME)
}

/// 从 0.1.7 起的 web 档案补丁中读取主题配置。
fn parse_patch_theme_preference(content: &str) -> Option<DshTheme> {
    let entries: Vec<serde_yaml::Value> = serde_yaml::from_str(content).ok()?;
    let theme_entry = entries
        .iter()
        .find(|entry| entry.get("id").and_then(serde_yaml::Value::as_str) == Some("ui-theme"))?;
    parse_preference(theme_entry.get("config")?.get("preference")?.as_str()?)
}

fn parse_preference(value: &str) -> Option<DshTheme> {
    match value {
        "light" => Some(DshTheme::Light),
        "dark" => Some(DshTheme::Dark),
        "system" => Some(DshTheme::System),
        _ => None,
    }
}

/// 从 settings.yaml 文本中提取 `ui-theme.preference`（light/dark/system）
fn parse_theme_preference(content: &str) -> Option<DshTheme> {
    let mut in_ui_theme = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("ui-theme:") {
            in_ui_theme = true;
            continue;
        }
        if !in_ui_theme {
            continue;
        }
        // ui-theme 段结束（遇到无缩进的新顶层 key）
        if !line.starts_with(' ') && !line.starts_with('\t') {
            return None;
        }
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            if let Some(value) = trimmed.strip_prefix("preference:") {
                return parse_preference(value.trim());
            }
        }
    }
    None
}

/// 把 dsh 主题偏好同步为主窗口的原生外观（仅 macOS）。
///
/// macOS 上窗口保留了原生标题栏（`decorations(true)` + `Overlay` + `hidden_title`），
/// 其材质与交通灯底色跟随「系统外观」，而内嵌 dsh 页面的亮/暗由前端
/// `html[data-theme]` 独立控制；两者不联动就会出现「内容已切亮色、顶部标题栏仍
/// 是暗色」的割裂（issue #93）。这里把偏好直接落到原生外观：`system` → 跟随系统，
/// `light` / `dark` → 强制指定，让原生 chrome 与 dsh 页面始终一致。
///
/// 非 macOS 平台关闭了窗口 decoration、整窗皆为前端 `data-theme`，无原生 chrome
/// 可同步，无需（也无从）调用。
#[cfg(target_os = "macos")]
pub fn apply_window_theme(app_handle: &AppHandle, theme: DshTheme) {
    let Some(window) = app_handle.get_webview_window("main") else {
        log::debug!("apply_window_theme: main window not built yet");
        return;
    };
    let appearance = match theme {
        DshTheme::System => None,
        DshTheme::Light => Some(tauri::Theme::Light),
        DshTheme::Dark => Some(tauri::Theme::Dark),
    };
    if let Err(err) = window.set_theme(appearance) {
        log::warn!("[theme] failed to sync native window appearance: {err}");
    }
}

/// 主题偏好变化时向前端推送 `dsh-theme-updated` 事件（仅在变化时触发一次）。
/// 变化时同步 macOS 原生窗口外观，与前端 `data-theme` 保持同源。
pub fn check_and_emit_theme(app_handle: &AppHandle) {
    let theme = get_dsh_theme(app_handle);
    let mut last = LAST_EMITTED
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap();
    if *last == Some(theme) {
        return;
    }
    *last = Some(theme);
    log::debug!("dsh theme preference changed: {:?}", theme);
    let _ = app_handle.emit("dsh-theme-updated", &theme);
    #[cfg(target_os = "macos")]
    apply_window_theme(app_handle, theme);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_theme_from_web_profile_patch() {
        let patch = "- id: unrelated\n  config:\n    preference: dark\n- id: ui-theme\n  name: '@deepseek-ai/dsh-client-ui-theme'\n  config:\n    preference: light\n";
        assert_eq!(parse_patch_theme_preference(patch), Some(DshTheme::Light));
        assert_eq!(
            parse_patch_theme_preference(&patch.replace("preference: light", "preference: system")),
            Some(DshTheme::System)
        );
        assert_eq!(
            parse_patch_theme_preference(&patch.replace("preference: light", "preference: dark")),
            Some(DshTheme::Dark)
        );
    }

    #[test]
    fn keeps_legacy_theme_fallback() {
        assert_eq!(
            parse_theme_preference("other:\n  preference: dark\nui-theme:\n  preference: light\n"),
            Some(DshTheme::Light)
        );
        assert_eq!(DEFAULT_THEME, DshTheme::Light);
    }
}
