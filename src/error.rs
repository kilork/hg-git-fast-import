#[derive(Debug)]
pub enum ErrorKind {
    HgParserFailure(hg_parser::ErrorKind),
    Source(crate::SourceRepositoryError),
    Target(crate::TargetRepositoryError),
    Encoding(std::str::Utf8Error),
    IO(std::io::Error),
    VerifyFailure(String),
}

impl From<hg_parser::ErrorKind> for ErrorKind {
    fn from(value: hg_parser::ErrorKind) -> Self {
        ErrorKind::HgParserFailure(value)
    }
}

impl From<std::str::Utf8Error> for ErrorKind {
    fn from(value: std::str::Utf8Error) -> Self {
        ErrorKind::Encoding(value)
    }
}

impl From<std::io::Error> for ErrorKind {
    fn from(value: std::io::Error) -> Self {
        ErrorKind::IO(value)
    }
}

impl From<crate::TargetRepositoryError> for ErrorKind {
    fn from(value: crate::TargetRepositoryError) -> Self {
        ErrorKind::Target(value)
    }
}

impl From<crate::SourceRepositoryError> for ErrorKind {
    fn from(value: crate::SourceRepositoryError) -> Self {
        ErrorKind::Source(value)
    }
}
