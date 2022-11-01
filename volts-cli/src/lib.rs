use std::{
    collections::{HashMap, HashSet},
    io::stdin,
    path::PathBuf,
};

use flate2::{write::GzEncoder, Compression};
use lapce_rpc::plugin::VoltMetadata;
use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use toml_edit::easy as toml;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct IconTheme {
    pub icon_theme: IconThemeConfig,
}

#[derive(Serialize, Deserialize)]
struct IconThemeConfig {
    pub ui: HashMap<String, String>,
    pub foldername: HashMap<String, String>,
    pub filename: HashMap<String, String>,
    pub extension: HashMap<String, String>,
}

pub fn publish() {
    println!("please paste the API Token you created on https://plugins.lapce.dev/");
    let mut token = String::new();
    stdin().read_line(&mut token).unwrap();

    let temp_dir = tempfile::tempdir().unwrap();
    let tar_gz_path = temp_dir.path().join("volt.tar.gz");

    {
        let tar_gz_file = std::fs::File::create(&tar_gz_path).unwrap();
        let encoder = GzEncoder::new(tar_gz_file, Compression::default());
        let mut tar = tar::Builder::new(encoder);

        let volt_path = PathBuf::from("volt.toml");
        if !volt_path.exists() {
            eprintln!("volt.toml doesn't exist");
            return;
        }

        let s = std::fs::read_to_string(&volt_path).unwrap();
        let volt: VoltMetadata = match toml::from_str(&s) {
            Ok(volt) => volt,
            Err(_) => {
                eprintln!("volt.tmol format invalid");
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

                let s = std::fs::read_to_string(&theme_path).unwrap();
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
        .body(std::fs::File::open(&tar_gz_path).unwrap())
        .send()
        .unwrap();
    if resp.status() == StatusCode::OK {
        println!("plugin published successfully");
        return;
    }

    eprintln!("{}", resp.text().unwrap());
}
