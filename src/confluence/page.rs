use serde::Serialize;
use std::vec::Vec;
use tracing::{instrument, Level};

use crate::config::Page;

#[derive(Serialize, Debug)]
pub struct ConfluencePage {
    pub title: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub status: String,
    pub version: Version,
    pub body: Body,
    pub metadata: Metadata,
}

#[derive(Serialize, Debug)]
pub struct Version {
    pub number: u64,
}

#[derive(Serialize, Debug)]
pub struct Body {
    pub storage: Storage,
}

#[derive(Serialize, Debug)]
pub struct Storage {
    pub value: String,
    pub representation: String,
}

#[derive(Serialize, Debug)]
pub struct Metadata {
    pub labels: Vec<Label>,
}

#[derive(Serialize, Debug)]
pub struct Label {
    pub name: String,
}

impl From<String> for Label {
    fn from(label: String) -> Self {
        Label {
            name: label.replace(' ', "-"),
        }
    }
}

impl ConfluencePage {
    #[instrument(ret(level = Level::TRACE))]
    pub fn new(page: &Page, version: u64) -> Self {
        Self {
            title: page.title.to_string(),
            type_field: "page".to_string(),
            status: "current".to_string(),
            version: Version { number: version },
            body: Body {
                storage: Storage {
                    value: page.html.html.to_string(),
                    representation: "storage".to_string(),
                },
            },
            metadata: Metadata {
                labels: page.labels.iter().cloned().map(Label::from).collect(),
            },
        }
    }

    pub fn add_labels(mut self, labels: Vec<String>) -> Self {
        self.metadata
            .labels
            .extend(labels.iter().cloned().map(Label::from));
        self
    }
}
