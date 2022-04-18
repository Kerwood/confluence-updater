use reqwest::Error as ReqwestError;
use serde_yaml::Error as SerdeYamlError;
use std::error::Error;
use std::fmt;
use std::io::Error as IoError;

#[derive(Debug)]
pub enum ConUpdaterError {
    Io(IoError),
    SerdeYaml(SerdeYamlError),
    Reqwest(ReqwestError),
    BadStatusCode(String),
}

impl Error for ConUpdaterError {}

impl fmt::Display for ConUpdaterError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConUpdaterError::Io(e) => write!(f, "{}", e.to_string()),
            ConUpdaterError::SerdeYaml(e) => write!(f, "{}", e.to_string()),
            ConUpdaterError::Reqwest(e) => write!(f, "{}", e.to_string()),
            ConUpdaterError::BadStatusCode(e) => write!(f, "{}", e.to_string()),
        }
    }
}

impl From<std::io::Error> for ConUpdaterError {
    fn from(error: std::io::Error) -> Self {
        ConUpdaterError::Io(error)
    }
}

impl From<serde_yaml::Error> for ConUpdaterError {
    fn from(error: serde_yaml::Error) -> Self {
        ConUpdaterError::SerdeYaml(error)
    }
}

impl From<reqwest::Error> for ConUpdaterError {
    fn from(error: reqwest::Error) -> Self {
        ConUpdaterError::Reqwest(error)
    }
}

impl From<ConUpdaterError> for String {
    fn from(error: ConUpdaterError) -> Self {
        error.to_string()
    }
}
