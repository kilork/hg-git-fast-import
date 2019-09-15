use failure::Fail;

#[derive(Fail, Debug)]
pub enum ErrorKind {
    #[fail(display = "lib parser {}", _0)]
    HgParserFailure(hg_parser::ErrorKind),
    #[fail(display = "source error {}", _0)]
    Source(crate::SourceRepositoryError),
    #[fail(display = "target error {}", _0)]
    Target(crate::TargetRepositoryError),
    #[fail(display = "encoding error {}", _0)]
    Encoding(std::str::Utf8Error),
    #[fail(display = "io error {}", _0)]
    IO(std::io::Error),
    #[fail(display = "verify error {}", _0)]
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
