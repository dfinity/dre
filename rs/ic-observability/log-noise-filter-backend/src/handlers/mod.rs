use std::{collections::BTreeMap, net::SocketAddr, sync::Arc};

use axum::{routing::delete, Router};
use delete::delete_criteria;
use get::get_criteria;
use slog::Logger;
use tokio::sync::Mutex;

use self::put::update;
use axum::routing::{get, put};

mod delete;
mod get;
mod put;

#[derive(Clone)]
pub struct Server {
    pub logger: Logger,
    criteria: Arc<Mutex<Vec<String>>>,
}

impl Server {
    pub fn new(logger: Logger) -> Self {
        Self {
            logger,
            criteria: Arc::new(Mutex::new(vec![])),
        }
    }

    pub async fn run(&self, socket: SocketAddr) {
        let app = Router::new()
            .route("/", get(get_criteria))
            .route("/", put(update))
            .route("/", delete(delete_criteria))
            .with_state(self.clone());
        let listener = tokio::net::TcpListener::bind(socket).await.unwrap();
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                tokio::signal::ctrl_c().await.unwrap();
            })
            .await
            .unwrap();
    }

    pub async fn get_criteria_mapped(&self) -> BTreeMap<u32, String> {
        let criteria = self.criteria.lock().await;
        criteria.iter().enumerate().map(|(i, s)| (i as u32, s.clone())).collect()
    }

    pub async fn update_criteria(&self, criteria: Vec<String>) {
        let mut server_criteria = self.criteria.lock().await;
        criteria.iter().for_each(|c| {
            server_criteria.push(c.to_string());
        });
    }

    pub async fn delete_criteria(&self, mut indexes: Vec<u32>) -> Result<(), Vec<u32>> {
        let mut server_criteria = self.criteria.lock().await;
        indexes.sort_by(|a, b| b.cmp(a));
        let missing = indexes
            .iter()
            .filter_map(|c| match server_criteria.get(*c as usize) {
                Some(_) => None,
                None => Some(*c),
            })
            .collect::<Vec<_>>();

        if !missing.is_empty() {
            return Err(missing);
        }

        indexes.iter().for_each(|c| {
            server_criteria.remove(*c as usize);
        });

        Ok(())
    }
}
