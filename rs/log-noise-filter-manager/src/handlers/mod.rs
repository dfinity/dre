use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use axum::Router;
use serde::{Deserialize, Serialize};
use slog::{error, info, Logger};
use tokio::sync::Mutex;

use self::{
    get::{content, only_routes},
    put::update,
};
use axum::routing::{get, put};

mod get;
mod put;

pub const SEPARATOR: &str = " || ";

#[derive(Clone)]
pub struct Server {
    pub logger: Logger,
    pub file_path: PathBuf,
    mutex: Arc<Mutex<()>>,
}

impl Server {
    pub fn new(logger: Logger, file_path: PathBuf) -> Self {
        Self {
            logger,
            file_path,
            mutex: Arc::new(Mutex::new(())),
        }
    }

    pub async fn run(&self, socket: SocketAddr, reroute_unmatched: String, inputs: Vec<String>) {
        self.ensure_file_exists(reroute_unmatched, inputs).await;

        let app = Router::new()
            .route("/", get(content))
            .route("/only-routes", get(only_routes))
            .route("/", put(update))
            .with_state(self.clone());
        let listener = tokio::net::TcpListener::bind(socket).await.unwrap();
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                tokio::signal::ctrl_c().await.unwrap();
            })
            .await
            .unwrap();
    }

    async fn ensure_file_exists(&self, reroute_unmatched: String, inputs: Vec<String>) {
        match tokio::fs::File::open(&self.file_path).await {
            Ok(_) => {
                let _ = self.read_file().await;
                info!(self.logger, "Validated initial toml content");
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => match tokio::fs::File::create(&self.file_path).await {
                Ok(_) => {
                    if inputs.is_empty() {
                        error!(self.logger, "Vector needs at least one input.");
                        panic!()
                    }
                    self.write_structure(&Self::get_initial(reroute_unmatched, inputs)).await;
                    info!(self.logger, "Serialized initial structure")
                }
                Err(e) => {
                    error!(self.logger, "Received an error while creating the file: {:?}", e);
                    panic!();
                }
            },
            Err(e) => {
                error!(self.logger, "Received unexpected error: {:?}", e);
                panic!();
            }
        };
    }

    fn get_initial(reroute_unmatched: String, inputs: Vec<String>) -> TopLevelVectorTransform {
        TopLevelVectorTransform {
            transforms: VectorTransform {
                noise_filter: NoiseFilter {
                    type_: "route".to_string(),
                    inputs,
                    reroute_unmatched,
                    route: Route { noisy: "false".to_string() },
                },
            },
        }
    }

    pub async fn write_structure(&self, structure: &TopLevelVectorTransform) {
        let serialized = match toml::to_string_pretty(&structure) {
            Ok(v) => v,
            Err(e) => {
                error!(self.logger, "Error while serializing initial structure: {:?}", e);
                panic!();
            }
        };
        match tokio::fs::write(&self.file_path, &serialized.as_bytes()).await {
            Ok(_) => {}
            Err(e) => {
                error!(self.logger, "Couldn't serialize initial strucuture: {:?}", e);
                panic!()
            }
        }
    }

    pub async fn read_file(&self) -> TopLevelVectorTransform {
        let _ = self.mutex.lock().await;
        let content = match tokio::fs::read_to_string(&self.file_path).await {
            Ok(c) => c,
            Err(e) => {
                error!(self.logger, "Couldn't read content to string: {:?}", e);
                panic!()
            }
        };

        match toml::from_str::<TopLevelVectorTransform>(&content) {
            Ok(v) => v,
            Err(e) => {
                error!(self.logger, "Validation of initial toml failed: {:?}", e);
                panic!()
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct TopLevelVectorTransform {
    pub transforms: VectorTransform,
}
#[derive(Serialize, Deserialize)]

pub struct VectorTransform {
    pub noise_filter: NoiseFilter,
}
#[derive(Serialize, Deserialize)]
pub struct NoiseFilter {
    pub type_: String,
    pub inputs: Vec<String>,
    pub reroute_unmatched: String,
    pub route: Route,
}
#[derive(Serialize, Deserialize)]
pub struct Route {
    pub noisy: String,
}
