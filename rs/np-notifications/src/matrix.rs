use std::cell::RefCell;

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Debug)]
pub enum ClientError {
    Authentication(String),
    Query(reqwest::Error),
    Parsing(reqwest::Error),
    MissingCredentials(String),
    Decoding(reqwest::Error),
    UnknownResponse(String),
}

pub struct Client {
    client: reqwest::Client,
    url: url::Url,
    // Would be used in case we need to reauthenticate from scratch
    // For now, it trigger clippy, so prefixing it.
    _credentials: Credentials,
    authentication_token: AuthenticationToken,
    // We need this internally in the Client, but it should not be visible.
    // We internally mutate it to achieve that.
    // https://doc.rust-lang.org/book/ch15-05-interior-mutability.html
    transaction_id: RefCell<usize>,
}

impl Client {
    pub async fn new(url: String, credentials: Credentials) -> Result<Self, ClientError> {
        let client = reqwest::Client::new();
        let auth_endpoint = "/_matrix/client/v3/login";
        let url = url::Url::parse(&url).unwrap();
        let response = client
            .post(url.join(auth_endpoint).unwrap().as_str())
            .json(&LoginRequest {
                _type: "m.login.password".to_string(),
                identifier: LoginRequestIdentifier {
                    _type: "m.id.user".to_string(),
                    user: credentials.username.clone(),
                },
                password: credentials.password.clone(),
            })
            .send()
            .await
            .map_err(ClientError::Query)?;

        #[derive(Deserialize)]
        pub struct AuthenticationError {
            #[serde(rename = "errcode")]
            _errcode: String,
            error: String,
        }

        match response.status() {
            StatusCode::OK => {
                let authentication_token: AuthenticationToken = response.json().await.map_err(ClientError::Parsing)?;
                Ok(Self {
                    client,
                    url,
                    _credentials: credentials,
                    authentication_token,
                    transaction_id: RefCell::new(0),
                })
            }
            StatusCode::FORBIDDEN => Err(ClientError::Authentication(
                response
                    .json::<AuthenticationError>()
                    .await
                    .map_err(ClientError::Parsing)?
                    .error,
            )),
            _ => Err(ClientError::UnknownResponse(
                response.text().await.map_err(ClientError::Decoding)?,
            )),
        }
    }

    pub async fn from_config(config: Config) -> Result<Self, ClientError> {
        Self::new(
            config.matrix.instance.to_string(),
            Credentials {
                username: config
                    .matrix
                    .username
                    .ok_or(ClientError::MissingCredentials("Missing Matrix username".to_string()))?,
                password: config
                    .matrix
                    .password
                    .ok_or(ClientError::MissingCredentials("Missing Matrix password".to_string()))?,
            },
        )
        .await
    }

    async fn put(&self, endpoint: &str, body: impl Serialize) -> Result<reqwest::Response, reqwest::Error> {
        self.client
            .put(self.url.join(endpoint).unwrap())
            .header(
                "Authorization",
                format!("Bearer {}", self.authentication_token.access_token),
            )
            .json(&body)
            .send()
            .await
    }

    pub async fn send_message_to_room(&self, room_id: String, message: String) -> Result<String, ClientError> {
        // Needed if we want to send multiple different messages with idempotency.
        // We keep a local copy so that we can retry the transaction
        let transaction_id = *self.transaction_id.borrow();
        // We then bump the id if we need to make other queries
        self.transaction_id.replace(transaction_id + 1);
        let endpoint = format!(
            "/_matrix/client/v3/rooms/{}/send/{}/{}",
            room_id, "m.room.message", transaction_id
        );

        self.put(&endpoint, &MatrixMessage::new(message))
            .await
            .map_err(ClientError::Query)?
            .text()
            .await
            .map_err(ClientError::Query)
    }
}

#[derive(Deserialize)]
struct AuthenticationToken {
    access_token: String,
}

#[derive(Clone)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
struct LoginRequest {
    #[serde(rename = "type")]
    _type: String,
    identifier: LoginRequestIdentifier,
    password: String,
}

// Only valid for the user type
#[derive(Serialize)]
struct LoginRequestIdentifier {
    #[serde(rename = "type")]
    _type: String,
    user: String,
}

#[derive(Serialize)]
struct MatrixMessageEventGenerator {
    #[serde(rename = "roomId")]
    room_id: String,
    #[serde(rename = "eventType")]
    event_type: String, // "m.room.message"
    #[serde(rename = "txnId")]
    txn_id: usize,
}

#[derive(Serialize)]
struct MatrixMessageEvent {
    #[serde(rename = "roomId")]
    room_id: String,
    #[serde(rename = "eventType")]
    event_type: String, // "m.room.message"
    #[serde(rename = "txnId")]
    txn_id: usize,
}

#[derive(Serialize)]
struct MatrixMessage {
    body: String,
    msgtype: &'static str,
}
impl MatrixMessage {
    fn new(body: String) -> Self {
        Self {
            body,
            msgtype: "m.text",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use ic_types::PrincipalId;

    use crate::{config, matrix::Client};

    // To run this test, define the required environment variables (see the
    // variables set by `get_var`), then run
    // `cargo run -- --ignored`
    #[actix_web::test]
    #[ignore]
    async fn matrix_test() {
        fn get_var(name: &str) -> String {
            env::var(name).unwrap_or_else(|_| panic!("To run this test, you should define {}", name))
        }
        let room_id = get_var("NP_MATRIX_ROOM_ID");
        let config = config::Config::new().unwrap();
        let client = Client::from_config(config).await.unwrap();

        let notification = crate::notification::Notification {
            node_id: PrincipalId::new_node_test_id(0),
            node_provider: None,
            status_change: (
                ic_management_types::Status::Healthy,
                ic_management_types::Status::Degraded,
            ),
        };
        let response = client.send_message_to_room(room_id, notification.to_string()).await;
        assert!(response.is_ok());
    }
}
