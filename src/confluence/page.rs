use crate::config::Page;
use regex::Regex;
use serde::Serialize;
use std::vec::Vec;
use tracing::{instrument, warn, Level};

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
        Label { name: label }
    }
}

impl ConfluencePage {
    #[instrument(skip_all, ret(level = Level::TRACE))]
    pub fn new(page: &Page, version: u64) -> Self {
        let labels = filter_valid_labels(&page.labels)
            .into_iter()
            .map(Label::from)
            .collect();

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
            metadata: Metadata { labels },
        }
    }

    pub fn add_labels(mut self, labels: Vec<String>) -> Self {
        self.metadata
            .labels
            .extend(labels.iter().cloned().map(Label::from));
        self
    }
}

pub fn filter_valid_labels(labels: &[String]) -> Vec<String> {
    let pattern = r"^[a-z0-9\$%'+\-/=\_`{}|~]+$";
    let regex = Regex::new(pattern).expect("invalid regex patteren");

    let filter_labels = |label| match regex.is_match(label) {
        true => Some(label.to_string()),
        false => {
            warn!(
                "invalid label [{}]. labels must match the following regex: {}, skipping..",
                label, pattern
            );
            None
        }
    };

    labels.iter().filter_map(|x| filter_labels(x)).collect()
}
