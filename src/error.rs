#[derive(Debug, thiserror::Error)]
pub enum ErrorKind {
    #[error("lib parser {0}")]
    HgParserFailure(#[from] hg_parser::ErrorKind),
    #[error("source error {0}")]
    Source(#[from] crate::SourceRepositoryError),
    #[error("target error {0}")]
    Target(#[from] crate::TargetRepositoryError),
    #[error("encoding error {0}")]
    Encoding(#[from] std::str::Utf8Error),
    #[error("io error {0}")]
    IO(#[from] std::io::Error),
    #[error("verify error {0}")]
    VerifyFailure(String),
    #[error("wrong file data {0}")]
    WrongFileData(String),
    #[error(
        "wrong name of Mercurial user '{0}'.
Must be in form 'Username <username@email.xyz>'.
Use --authors option to specify mapping file in TOML format.
Or use [authors] section in config.

Example:

    '{0}' = 'My <my@normal.xyz>'

will replace Mercurial '{0}' with 'My <my@normal.xyz>' in Git.
"
    )]
    WrongUser(String),
    #[error(transparent)]
    TemplateError(#[from] indicatif::style::TemplateError),
    #[error(transparent)]
    DialoguerError(#[from] dialoguer::Error),
}
