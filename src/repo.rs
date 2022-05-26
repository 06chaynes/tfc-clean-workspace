use std::path::Path;

use crate::{error::AppError, settings::Settings};
use git2::{build::RepoBuilder, Repository};
use git2_credentials::CredentialHandler;
use log::*;
use url::Url;

pub fn clone(
    config: &Settings,
    url: Url,
    identifier: String,
) -> Result<(), AppError> {
    let mut cb = git2::RemoteCallbacks::new();
    let git_config = git2::Config::open_default().unwrap();
    let mut ch = CredentialHandler::new(git_config);
    cb.credentials(move |url, username, allowed| {
        ch.try_next_credential(url, username, allowed)
    });
    let mut fo = git2::FetchOptions::new();
    fo.remote_callbacks(cb).update_fetchhead(true);

    let mut base_dir = config.repositories.git_dir.clone();
    if base_dir.ends_with('/') {
        base_dir.pop();
    }
    let path = format!("{}/{}", base_dir, identifier);
    info!("Cloning repo: {} into {}", url.as_str(), &path);
    let path = Path::new(&path);
    if path.is_dir() {
        info!("Repo already exists locally, updating instead.");
        // Open and Update
        match Repository::open(path) {
            Ok(repo) => {
                info!("Updating repo.");
                match repo.checkout_head(None) {
                    Ok(_) => info!("Repo at HEAD!"),
                    Err(e) => {
                        error!("Checkout HEAD failed :(");
                        return Err(AppError::Git(e));
                    }
                }
            }
            Err(e) => {
                error!("Open failed :(");
                return Err(AppError::Git(e));
            }
        }
    } else {
        // Clone
        match RepoBuilder::new().fetch_options(fo).clone(url.as_str(), path) {
            Ok(_repo) => info!("Clone successful!"),
            Err(e) => {
                error!("Clone failed :(");
                return Err(AppError::Git(e));
            }
        }
    }
    Ok(())
}
