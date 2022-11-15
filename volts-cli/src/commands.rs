use std::{
    collections::HashSet,
    fs::{self, File},
    path::PathBuf,
};

use lapce_rpc::plugin::VoltMetadata;
use reqwest::{Method, StatusCode};
use tar::Builder;
use toml_edit::easy as toml;
use zstd::Encoder;

use crate::{auth_token, Cli, IconTheme};

pub(crate) fn publish(cli: &Cli) {
    let token = auth_token(cli);

    let temp_dir = tempfile::tempdir().unwrap();
    let archive_path = temp_dir.path().join("plugin.volt");

    {
        let archive = File::create(&archive_path).unwrap();
        let encoder = Encoder::new(archive, 0).unwrap();
        let mut tar = Builder::new(encoder);

        let volt_path = PathBuf::from("volt.toml");
        if !volt_path.exists() {
            eprintln!("volt.toml doesn't exist");
            return;
        }

        let s = fs::read_to_string(&volt_path).unwrap();
        let volt: VoltMetadata = match toml::from_str(&s) {
            Ok(volt) => volt,
            Err(e) => {
                eprintln!("volt.toml format invalid: {e}");
                return;
            }
        };

        tar.append_path(&volt_path).unwrap();

        if let Some(wasm) = volt.wasm.as_ref() {
            let wasm_path = PathBuf::from(wasm);
            if !wasm_path.exists() {
                eprintln!("wasm {wasm} not found");
                return;
            }

            tar.append_path(&wasm_path).unwrap();
        } else if let Some(themes) = volt.color_themes.as_ref() {
            if themes.is_empty() {
                eprintln!("no color theme provided");
                return;
            }
            for theme in themes {
                let theme_path = PathBuf::from(theme);
                if !theme_path.exists() {
                    eprintln!("color theme {theme} not found");
                    return;
                }

                tar.append_path(&theme_path).unwrap();
            }
        } else if let Some(themes) = volt.icon_themes.as_ref() {
            if themes.is_empty() {
                eprintln!("no icon theme provided");
                return;
            }
            for theme in themes {
                let theme_path = PathBuf::from(theme);
                if !theme_path.exists() {
                    eprintln!("icon theme {theme} not found");
                    return;
                }

                tar.append_path(&theme_path).unwrap();

                let s = fs::read_to_string(&theme_path).unwrap();
                let theme_config: IconTheme = match toml::from_str(&s) {
                    Ok(config) => config,
                    Err(_) => {
                        eprintln!("icon theme {theme} format invalid");
                        return;
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
                        eprintln!("icon {icon} not found");
                        return;
                    }
                    tar.append_path(&icon_path).unwrap();
                }
            }
        } else {
            eprintln!("not a valid plugin");
            return;
        }

        let readme_path = PathBuf::from("README.md");
        if readme_path.exists() {
            tar.append_path(&readme_path).unwrap();
        }

        if let Some(icon) = volt.icon.as_ref() {
            let icon_path = PathBuf::from(icon);
            if !icon_path.exists() {
                eprintln!("icon not found at the specified path");
                return;
            }
            tar.append_path(&icon_path).unwrap();
        }
        tar.finish().unwrap();
    }

    let resp = reqwest::blocking::Client::new()
        .request(
            Method::PUT,
            "https://plugins.lapce.dev/api/v1/me/plugins/new",
        )
        .bearer_auth(token.trim())
        .body(File::open(&archive_path).unwrap())
        .send()
        .unwrap();
    if resp.status() == StatusCode::OK {
        println!("plugin published successfully");
        return;
    }

    eprintln!("{}", resp.text().unwrap());
}

pub(crate) fn yank(cli: &Cli, name: &String, version: &String) {
    let token = auth_token(cli);

    let resp = reqwest::blocking::Client::new()
        .request(
            Method::PUT,
            format!("https://plugins.lapce.dev/api/v1/me/plugins/{name}/{version}/yank"),
        )
        .bearer_auth(token.trim())
        .send()
        .unwrap();
    if resp.status() == StatusCode::OK {
        println!("plugin version yanked successfully");
    } else {
        eprintln!("failed to yank plugin version: {}", resp.text().unwrap());
    }
}

pub(crate) fn unyank(cli: &Cli, name: &String, version: &String) {
    let token = auth_token(cli);

    let resp = reqwest::blocking::Client::new()
        .request(
            Method::PUT,
            format!("https://plugins.lapce.dev/api/v1/me/plugins/{name}/{version}/unyank"),
        )
        .bearer_auth(token.trim())
        .send()
        .unwrap();
    if resp.status() == StatusCode::OK {
        println!("plugin version yanked successfully");
    } else {
        eprintln!("failed to yank plugin version: {}", resp.text().unwrap());
    }
}
