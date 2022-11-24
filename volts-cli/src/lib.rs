mod commands;

use std::{collections::HashMap, io::stdin};

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

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
    Yank { name: String, version: String },
    /// Undo yanking version from registry
    Unyank { name: String, version: String },
}

pub fn cli() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Publish {} => commands::publish(&cli),
        Commands::Yank { name, version } => commands::yank(&cli, name, version),
        Commands::Unyank { name, version } => commands::unyank(&cli, name, version),
    }
}

fn auth_token(cli: &Cli) -> String {
    if let Some(token) = &cli.token {
        token.to_owned()
    } else {
        let api_credential = keyring::Entry::new("lapce-volts", "registry-api");
        if let Ok(token) = api_credential.get_password() {
            return token;
        }

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
    }
}
