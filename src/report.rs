use crate::{
    error::AppError,
    settings::{Query, Settings},
    variable,
    workspace::{Workspace, WorkspaceVariables},
};
use std::fs::File;

use log::*;
use serde::{Deserialize, Serialize};

// For now need to keep this updated with best effort :)
const REPORT_VERSION: &str = "0.1.0";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Variable {
    pub id: String,
    pub key: String,
}

impl From<variable::Variable> for Variable {
    fn from(item: variable::Variable) -> Self {
        Variable { id: item.id, key: item.attributes.key }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UnlistedVariables {
    pub workspace: WorkspaceVariables,
    pub unlisted_variables: Vec<Variable>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Report {
    pub report_version: String,
    pub bin_version: String,
    pub query: Option<Query>,
    pub workspaces: Vec<Workspace>,
    pub missing_repositories: Vec<Workspace>,
    pub unlisted_variables: Vec<UnlistedVariables>,
}

impl Default for Report {
    fn default() -> Self {
        Self {
            report_version: REPORT_VERSION.to_string(),
            bin_version: env!("CARGO_PKG_VERSION").to_string(),
            query: None,
            workspaces: vec![],
            missing_repositories: vec![],
            unlisted_variables: vec![],
        }
    }
}

pub fn save(config: &Settings, report: Report) -> Result<(), AppError> {
    info!("Saving report to: {}", &config.output);
    match serde_json::to_writer_pretty(&File::create(&config.output)?, &report)
    {
        Ok(_) => {
            info!("Report Saved!");
        }
        Err(e) => {
            error!("Failed to save report!");
            return Err(AppError::Json(e));
        }
    }
    Ok(())
}
