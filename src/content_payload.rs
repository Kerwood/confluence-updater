use crate::page::Page;
use serde::{Deserialize, Serialize};
use std::vec::Vec;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ContentPayload {
    pub title: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub status: String,
    pub version: Version,
    pub body: Body,
    pub metadata: Metadata,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub number: u64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Body {
    pub storage: Storage,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Storage {
    pub value: String,
    pub representation: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub labels: Vec<Label>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Label {
    pub name: String,
}

impl ContentPayload {
    pub fn new(page: &Page, version: u64, labels: &Vec<String>, html: &str) -> Self {
        Self {
            title: page.title.to_owned(),
            type_field: page.content_type.to_owned(),
            status: "current".to_owned(),
            version: Version {
                number: version.to_owned(),
            },
            body: Body {
                storage: Storage {
                    value: html.to_owned(),
                    representation: "storage".to_owned(),
                },
            },
            metadata: Metadata {
                labels: labels
                    .iter()
                    .map(|x| Label::from(x))
                    .collect::<Vec<Label>>(),
            },
        }
    }
}

impl From<&String> for Label {
    fn from(label: &String) -> Self {
        Label {
            name: label.to_owned(),
        }
    }
}

#[derive(Deserialize)]
pub struct LabelRoot {
    pub results: Vec<Label>,
}
