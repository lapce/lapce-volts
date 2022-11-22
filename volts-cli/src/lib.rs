mod commands;

use std::{collections::HashMap, io::stdin, ops::Deref};

use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use lapce_rpc::plugin::VoltMetadata;
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

#[derive(Parser)]
#[clap(version, name = "Volts")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Registry API authentication token
    #[clap(long, action)]
    token: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Publish plugin to registry
    Publish {},
    /// Yank version from registry
    Yank {
        author: Option<String>,
        name: Option<String>,
        version: String,
    },
    /// Undo yanking version from registry
    Unyank {
        author: Option<String>,
        name: Option<String>,
        version: String,
    },
}

pub fn cli() {
    let cli = Cli::parse();

    if let Err(e) = match &cli.command {
        Commands::Publish {} => commands::publish(&cli),
        Commands::Yank {
            author,
            name,
            version,
        } => {
            if author.is_none() || name.is_none() {
                let volt = read_volt();
                commands::yank(&cli, volt.author, volt.name, version)
            } else {
                commands::yank(&cli, &author.unwrap(), &name.unwrap(), version)
            }
        },
        Commands::Unyank {
            author,
            name,
            version,
        } => commands::unyank(&cli, author, name, version),
    } {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

const VOLT_MANIFEST: &str = "volt.toml";

fn read_volt() -> Result<VoltMetadata> {
    let workdir = std::env::current_dir()?;
    let volt_path = workdir.join(VOLT_MANIFEST);
    if !volt_path.exists() {
        return Err(anyhow!("{VOLT_MANIFEST} doesn't exist"));
    }

    let s = std::fs::read_to_string(&volt_path)?;
    let volt = match toml::from_str::<VoltMetadata>(&s) {
        Ok(mut volt) => {
            volt
        }
        Err(_) => {
            return Err(anyhow!("{VOLT_MANIFEST} format invalid"));
        }
    };

    if semver::Version::parse(&volt.version).is_err() {
        return Err(anyhow!("version isn't valid"));
    }

    Ok(volt)
}

fn auth_token(cli: &Cli) -> String {
    let api_credential = keyring::Entry::new("lapce-volts", "registry-api");

    return if cli.token.is_none() && api_credential.get_password().is_err() {
        println!("Please paste the API Token you created on https://plugins.lapce.dev/");
        let mut token = String::new();
        stdin().read_line(&mut token).unwrap();

        token = token.trim().to_string();
        if token.is_empty() {
            eprintln!("Token cannot be empty");
            std::process::exit(1);
        }

        if let Err(why) = api_credential.set_password(&token) {
            eprintln!("Failed to save token in system credential store: {why}");
        };

        token
    } else if let Some(token) = &cli.token {
        if api_credential.get_password().is_err() {
            if let Err(why) = api_credential.set_password(token) {
                eprintln!("Failed to save token in system credential store: {why}");
            };
        }
        token.to_owned()
    } else {
        api_credential.get_password().unwrap()
    };
}
