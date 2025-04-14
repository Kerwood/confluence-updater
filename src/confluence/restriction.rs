use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Restriction {
    results: Vec<Result>,
}

#[derive(Debug, Serialize)]
struct Result {
    operation: String,
    restrictions: Restrictions,
}

#[derive(Debug, Serialize)]
struct Restrictions {
    pub user: Vec<User>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct User {
    #[serde(rename = "type")]
    type_field: String,
    account_id: String,
}

impl Restriction {
    pub fn read_only(account_id: &str) -> Self {
        Self {
            results: vec![Result {
                operation: "update".to_string(),
                restrictions: Restrictions {
                    user: vec![User {
                        type_field: "known".to_string(),
                        account_id: account_id.to_string(),
                    }],
                },
            }],
        }
    }

    pub fn no_restrictions() -> Self {
        Self {
            results: vec![Result {
                operation: "update".to_string(),
                restrictions: Restrictions { user: vec![] },
            }],
        }
    }
}
