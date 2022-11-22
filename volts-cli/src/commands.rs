use std::{
    collections::HashSet,
    fs::{self, File},
    path::PathBuf,
};

use anyhow::{anyhow, Result};
use lapce_rpc::plugin::VoltMetadata;
use reqwest::{Method, StatusCode};
use tar::Builder;
use toml_edit::easy as toml;
use zstd::Encoder;

use crate::{auth_token, Cli, IconTheme};

pub(crate) fn publish(cli: &Cli) -> Result<()> {
    let token = auth_token(cli);

    let temp_dir = tempfile::tempdir()?;
    let archive_path = temp_dir.path().join("plugin.volt");

    {
        let archive = File::create(&archive_path)?;
        let encoder = Encoder::new(archive, 0)?;
        let mut tar = Builder::new(encoder);

        let volt_path = PathBuf::from("volt.toml");
        if !volt_path.exists() {
            return Err(anyhow!("volt.toml doesn't exist"));
        }

        let s = fs::read_to_string(&volt_path)?;
        let volt: VoltMetadata = match toml::from_str(&s) {
            Ok(volt) => volt,
            Err(e) => {
                return Err(anyhow!("volt.toml format invalid: {e}"));
            }
        };

        tar.append_path(&volt_path)?;

        if let Some(wasm) = volt.wasm.as_ref() {
            let wasm_path = PathBuf::from(wasm);
            if !wasm_path.exists() {
                return Err(anyhow!("wasm {wasm} not found"));
            }

            tar.append_path(&wasm_path)?;
        } else if let Some(themes) = volt.color_themes.as_ref() {
            if themes.is_empty() {
                return Err(anyhow!("no color theme provided"));
            }
            for theme in themes {
                let theme_path = PathBuf::from(theme);
                if !theme_path.exists() {
                    return Err(anyhow!("color theme {theme} not found"));
                }

                tar.append_path(&theme_path)?;
            }
        } else if let Some(themes) = volt.icon_themes.as_ref() {
            if themes.is_empty() {
                return Err(anyhow!("no icon theme provided"));
            }
            for theme in themes {
                let theme_path = PathBuf::from(theme);
                if !theme_path.exists() {
                    return Err(anyhow!("icon theme {theme} not found"));
                }

                tar.append_path(&theme_path)?;

                let s = fs::read_to_string(&theme_path)?;
                let theme_config: IconTheme = match toml::from_str(&s) {
                    Ok(config) => config,
                    Err(_) => {
                        return Err(anyhow!("icon theme {theme} format invalid"));
                    }
                };

                let mut icons = HashSet::new();
                icons.extend(theme_config.icon_theme.ui.values());
                icons.extend(theme_config.icon_theme.filename.values());
                icons.extend(theme_config.icon_theme.foldername.values());
                icons.extend(theme_config.icon_theme.extension.values());

                let cwd = PathBuf::from(".");

                for icon in icons {
                    let icon_path = theme_path.parent().unwrap_or(&cwd).join(icon);
                    if !icon_path.exists() {
                        return Err(anyhow!("icon {icon} not found"));
                    }
                    tar.append_path(&icon_path)?;
                }
            }
        } else {
            return Err(anyhow!("not a valid plugin"));
        }

        let readme_path = PathBuf::from("README.md");
        if readme_path.exists() {
            tar.append_path(&readme_path)?;
        }

        if let Some(icon) = volt.icon.as_ref() {
            let icon_path = PathBuf::from(icon);
            if !icon_path.exists() {
                return Err(anyhow!("icon not found at the specified path"));
            }
            tar.append_path(&icon_path)?;
        }
        tar.finish()?;
    }

    let resp = reqwest::blocking::Client::new()
        .request(
            Method::PUT,
            "https://plugins.lapce.dev/api/v1/me/plugins/new",
        )
        .bearer_auth(token.trim())
        .body(File::open(&archive_path).unwrap())
        .send()?;
    if resp.status() == StatusCode::OK {
        return Err(anyhow!("plugin published successfully"));
    }

    eprintln!("{}", resp.text()?);

    Ok(())
}

pub(crate) fn yank(cli: &Cli, author: &String, name: &String, version: &String) -> Result<()> {
    let token = auth_token(cli);

    let resp = reqwest::blocking::Client::new()
        .request(
            Method::PUT,
            format!("https://plugins.lapce.dev/api/v1/me/plugins/{name}/{version}/yank"),
        )
        .bearer_auth(token.trim())
        .send()?;
    if resp.status() == StatusCode::OK {
        println!("plugin version yanked successfully");
    } else {
        return Err(anyhow!(
            "failed to yank plugin version: {}",
            resp.text().unwrap()
        ));
    }

    Ok(())
}

pub(crate) fn unyank(cli: &Cli, author: &String, name: &String, version: &String) -> Result<()> {
    let token = auth_token(cli);

    let resp = reqwest::blocking::Client::new()
        .request(
            Method::PUT,
            format!("https://plugins.lapce.dev/api/v1/me/plugins/{name}/{version}/unyank"),
        )
        .bearer_auth(token.trim())
        .send()?;
    if resp.status() == StatusCode::OK {
        println!("plugin version yanked successfully");
    } else {
        return Err(anyhow!(
            "failed to unyank plugin version: {}",
            resp.text().unwrap()
        ));
    }

    Ok(())
}
