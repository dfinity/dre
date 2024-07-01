use std::{collections::BTreeMap, net::SocketAddr, sync::Arc};

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use criteria::{delete_criteria::delete_criteria, get_criteria::get_criteria, post_criteria::update};

use get_all::get_all;
use rate::{get_rate::get_rate, put_rate::put_rate};
use regex::Regex;
use slog::Logger;
use tokio::sync::Mutex;

mod criteria;
pub mod get_all;
mod rate;

#[derive(Clone)]
pub struct Server {
    pub logger: Logger,
    criteria: Arc<Mutex<Vec<String>>>,
    rate: Arc<Mutex<u64>>,
}

impl Server {
    pub fn new(logger: Logger, rate: u64) -> Self {
        Self {
            logger,
            criteria: Arc::new(Mutex::new(vec![])),
            rate: Arc::new(Mutex::new(rate)),
        }
    }

    pub async fn run(&self, socket: SocketAddr) {
        let app = Router::new()
            .route("/criteria", get(get_criteria))
            .route("/criteria", post(update))
            .route("/criteria", delete(delete_criteria))
            .route("/rate", get(get_rate))
            .route("/rate", put(put_rate))
            .route("/", get(get_all))
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

    pub async fn get_rate(&self) -> u64 {
        *self.rate.lock().await
    }

    pub async fn update_criteria(&self, mut criteria: Vec<String>) -> Result<(), Vec<String>> {
        let mut server_criteria = self.criteria.lock().await;
        let mut errors = vec![];
        for c in criteria.iter_mut() {
            *c = c.replace('\\', "\\\\");
            *c = c.replace('\'', "\\'");
            if let Err(e) = Regex::new(c) {
                errors.push(e.to_string());
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }
        criteria.into_iter().for_each(|c| server_criteria.push(c));
        Ok(())
    }

    pub async fn update_rate(&self, rate: u64) -> u64 {
        *self.rate.lock().await = rate;
        rate
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
