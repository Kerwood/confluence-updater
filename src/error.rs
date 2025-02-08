#[derive(thiserror::Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Error {
    #[error("ERROR: HTTP request, {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("ERROR: Could not read the config file: {0}")]
    IO(#[from] std::io::Error),

    #[error("ERROR: Cound not parse YAML in config file: {0}")]
    SerdeYml(#[from] serde_yml::Error),

    #[error("ERROR: Faild to parse string to interger: {0}")]
    ParseInt(#[from] std::num::ParseIntError),

    #[error("ERROR: Cound not parse UTF-8 byte vector to String: {0}")]
    FromUtf8(#[from] std::string::FromUtf8Error),

    #[error("ERROR: No h1 header found in top of page. Add a header or use the overrideTitle configuration")]
    PageHeaderMissing,

    #[error("ERROR: Protocol scheme is missing from FQDN: [{0}]")]
    ProtocolSchemeMissing(String),

    #[error("ERROR: Failed to get the local part of the current user email.")]
    CurrentUserEmailMissing,

    #[error(
        "ERROR: A confluence page ID annotation was found on a link, but could not be parsed [{0}]"
    )]
    LinkIdMissing(String),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
