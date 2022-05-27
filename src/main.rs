mod error;
mod filter;
mod repo;
mod settings;
mod variable;
mod workspace;

use std::{ffi::OsStr, fs, os::unix::prelude::OsStrExt};

use clap::{Parser, Subcommand};
use env_logger::Env;
use error::AppError;
use http_cache_surf::{
    CACacheManager, Cache, CacheMode, CacheOptions, HttpCache,
};
use log::*;
use miette::{IntoDiagnostic, WrapErr};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use settings::Settings;
use surf::Client;
use surf_governor::GovernorMiddleware;
use url::Url;
use walkdir::{DirEntry, WalkDir};
use workspace::Workspace;

const BASE_URL: &str = "https://app.terraform.io/api/v2";

const ABOUT: &str =
    "Tool for rule based cleanup operations for Terraform workspaces";
const ABOUT_PLAN: &str = "Generates a report that contains information on the actions required to cleanup a workspace based on the provided rules";
const ABOUT_APPLY: &str =
    "Executes the actions described in the previously generated report";

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = Some(ABOUT))]
#[clap(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[clap(about = ABOUT_PLAN)]
    Plan,
    #[clap(about = ABOUT_APPLY)]
    Apply,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TestVariable {
    pub variable: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Variable {
    pub variable: Value,
}

fn build_governor() -> Result<GovernorMiddleware, AppError> {
    match GovernorMiddleware::per_second(30) {
        Ok(g) => Ok(g),
        Err(e) => Err(AppError::General(e.into_inner())),
    }
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name().to_str().map(|s| s.starts_with('.')).unwrap_or(false)
}

fn is_tf(entry: &DirEntry) -> bool {
    entry.path().extension() == Some(OsStr::from_bytes(b"tf"))
}

#[async_std::main]
async fn main() -> miette::Result<()> {
    // Parse cli subcommands and arguments
    let cli = Cli::parse();
    // Get the settings for the run
    let config = Settings::new()
        .into_diagnostic()
        .wrap_err("Uh Oh, looks like a settings issue! By default I look for a settings.toml file and override with env variables.")?;
    // Initialize the logger
    env_logger::Builder::from_env(
        Env::default().default_filter_or(&config.log),
    )
    .init();
    // Build the http client with a cache and governor enabled
    let client = Client::new().with(build_governor().into_diagnostic()?).with(
        Cache(HttpCache {
            mode: CacheMode::Default,
            manager: CACacheManager::default(),
            options: Some(CacheOptions {
                shared: false,
                cache_heuristic: 0.0,
                immutable_min_time_to_live: Default::default(),
                ignore_cargo_cult: false,
            }),
        }),
    );
    // Match on the cli subcommand
    match &cli.command {
        Commands::Plan => {
            info!("Start Plan Phase");
            // Get the initial list of workspaces
            let workspaces =
                workspace::get_workspaces(&config, client.clone()).await?;

            // Get the variables for each workspace
            let mut workspaces_variables = workspace::get_workspaces_variables(
                &config, client, workspaces,
            )
            .await?;
            // Filter the workspaces if query variables have been provided
            if config.query.variables.is_some() {
                info!("Filtering workspaces with variable query.");
                filter::variable(&mut workspaces_variables, &config)?;
            }

            let mut missing_repos: Vec<Workspace> = vec![];
            info!("Cloning workspace repositories.");
            for entry in &workspaces_variables {
                // Clone git repositories
                if let Some(repo) = &entry.workspace.attributes.vcs_repo {
                    info!(
                        "Repo detected for workspace: {}",
                        &entry.workspace.attributes.name
                    );
                    let url = Url::parse(&repo.repository_http_url)
                        .into_diagnostic()
                        .wrap_err("Failed to parse repository url")?;
                    let id = match repo.identifier.clone() {
                        Some(i) => i,
                        None => {
                            let segments = url.path_segments().unwrap();
                            segments.last().unwrap().to_string()
                        }
                    };
                    let mut base_dir = config.repositories.git_dir.clone();
                    if base_dir.ends_with('/') {
                        base_dir.pop();
                    }
                    let path = format!("{}/{}", base_dir, &id);
                    match repo::clone(
                        url.clone(),
                        path.clone(),
                        &entry.workspace,
                        &mut missing_repos,
                    ) {
                        Ok(_) => {}
                        Err(_e) => {}
                    };
                    info!("Parsing variable data.");
                    let walker = WalkDir::new(&path).into_iter();
                    for file in walker
                        .filter_entry(|e| !is_hidden(e))
                        .filter_map(Result::ok)
                        .filter(is_tf)
                    {
                        info!("Parsing file: {}", file.path().display());
                        match hcl::from_str::<TestVariable>(
                            &fs::read_to_string(file.path())
                                .into_diagnostic()?,
                        ) {
                            Ok(v) => {
                                info!("{:#?}", &v);
                            }
                            Err(_e) => {
                                match hcl::from_str::<Variable>(
                                    &fs::read_to_string(file.path())
                                        .into_diagnostic()?,
                                ) {
                                    Ok(value) => {
                                        for (key, value) in
                                            value.variable.as_object().unwrap()
                                        {
                                            info!(
                                                "{:#?} = {:#?}",
                                                &key, &value
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        warn!(
                                            "Error parsing file: {}",
                                            file.path().display()
                                        );
                                        warn!("{:#?}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            dbg!(missing_repos);
        }
        Commands::Apply => {
            dbg!("apply");
        }
    }
    Ok(())
}
