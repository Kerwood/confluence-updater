#[derive(thiserror::Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Error {
    #[error("HTTP request, {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Could not read the config file: {0}")]
    IO(#[from] std::io::Error),

    #[error("Cound not parse YAML in config file: {0}")]
    SerdeYml(#[from] serde_yml::Error),

    #[error("Faild to parse string to interger: {0}")]
    ParseInt(#[from] std::num::ParseIntError),

    #[error("Cound not parse UTF-8 byte vector to String: {0}")]
    FromUtf8(#[from] std::string::FromUtf8Error),

    #[error(
        "No h1 header found on top of page. Add a header or use the overrideTitle configuration"
    )]
    PageHeaderMissing,

    #[error("HTTPS protocol scheme missing from FQDN: [{0}]")]
    HttpsProtocolSchemeMissing(String),

    #[error("File path is invalid: [{0}]")]
    InvalidFilePath(String),

    #[error("Failed to get the local part of the current user email.")]
    CurrentUserEmailMissing,

    #[error("The link/path for image is missing")]
    ImageLinkMissing,

    #[error("A confluence page ID annotation was found on a link, but could not be parsed [{0}]")]
    LinkIdMissing(String),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
