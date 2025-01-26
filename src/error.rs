#[derive(thiserror::Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Error {
    #[error("ERROR: HTTP request, {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("ERROR: Could not read the config file. {0}")]
    IO(#[from] std::io::Error),

    #[error("ERROR: Cound not parse YAML in config file, {0}")]
    SerdeYml(#[from] serde_yml::Error),

    #[error("ERROR: Cound not parse FQDN to a valid url; {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("ERROR: Cound not parse UTF-8 byte vector to String; {0}")]
    FromUtf8(#[from] std::string::FromUtf8Error),

    #[error("ERROR: No h1 header found in top of page. Add a header or use the overrideTitle configuration")]
    PageHeaderMissing,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
