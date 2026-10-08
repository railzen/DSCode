//! 核心只随桌面安装包交付，不探测或修改用户的全局 npm 环境。
use crate::config;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarnessCore {
    pub id: String,
    pub source: String,
    pub version: String,
    pub path: String,
    pub dir: String,
    pub present: bool,
    pub active: bool,
}

/// 固定使用安装器管理的资源，不受旧设置或 PATH 影响。
pub fn active_dsh_binary(app: &AppHandle) -> PathBuf {
    config::get_dsh_binary_path(app)
}
pub fn active_version(app: &AppHandle) -> Option<String> {
    config::get_dsh_version(app)
}

/// 仅读取本地清单，离线打开核心页不发起网络请求。
pub async fn list(app: &AppHandle) -> Vec<HarnessCore> {
    let path = active_dsh_binary(app);
    vec![HarnessCore {
        id: "app".into(),
        source: "app".into(),
        version: active_version(app).unwrap_or_default(),
        path: path.to_string_lossy().into_owned(),
        dir: config::get_dsh_install_path(app)
            .to_string_lossy()
            .into_owned(),
        present: path.is_file(),
        active: true,
    }]
}

/// 仅查询官方仓库的 latest 标签；不可联网时静默省略，不依赖本机 npm。
pub async fn latest_version() -> Option<String> {
    #[derive(Deserialize)]
    struct LatestCore {
        version: String,
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let release = client
        .get("https://registry.npmjs.org/@deepseek-ai%2Fdsh/latest")
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json::<LatestCore>()
        .await
        .ok()?;
    semver::Version::parse(&release.version).ok()?;
    Some(release.version)
}
