use miette::Diagnostic;
use thiserror::Error;

/// A generic “error” type
#[derive(Error, Diagnostic, Debug)]
pub enum AppError {
    /// A general error used as a catch all for other errors via anyhow
    #[error(transparent)]
    #[diagnostic(code(clean_workspace::general))]
    General(#[from] anyhow::Error),
    /// URL parsing related errors
    #[error(transparent)]
    #[diagnostic(
        code(which_workspace::url),
        help("Oops, something went wrong building the URL!")
    )]
    Url(#[from] url::ParseError),
    /// JSON Serialization\Deserialization related errors
    #[error(transparent)]
    #[diagnostic(
        code(which_workspace::json),
        help("Aw snap, ran into an issue parsing the json response!")
    )]
    Json(#[from] serde_json::Error),
    /// Integer parsing related errors
    #[error(transparent)]
    #[diagnostic(
        code(which_workspace::int),
        help("Oh no, ran into an issue parsing an integer!")
    )]
    Int(#[from] std::num::ParseIntError),
    /// Git related errors
    #[error(transparent)]
    #[diagnostic(
        code(which_workspace::git),
        help("My bad, something went wrong with git!")
    )]
    Git(#[from] git2::Error),
    /// Walkdir related errors
    #[error(transparent)]
    #[diagnostic(
        code(which_workspace::walkdir),
        help("Oh Bother, something went walking the directory!")
    )]
    Walkdir(#[from] walkdir::Error),
    /// std IO related errors
    #[error(transparent)]
    #[diagnostic(code(which_workspace::io), help("Dangit, IO issue!"))]
    Io(#[from] std::io::Error),
}
