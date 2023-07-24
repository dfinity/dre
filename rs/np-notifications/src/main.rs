// The goal is to be able to notify Node Providers of changes in status of their
// nodes. Here are some elements that we might want to include
//
// - Fetch data from prometheus
// - Refresh state based on this data, maybe with a log of state changes
// - Send message on status change
// - Fetch latest known state on startup, to notify in case of a change during a restart
//
// We should also have an API to get the current status
// GET /api/v1/state/<node-id>
// GET /api/v1/state
//
// We should have a webhook API
// We want a way to register new webhooks
//
// We want to have a way to send random events in the service, and see them go through. Can be a webhook, a simple log sink.
//
// Questions to solve:
// What happens when the service restarts ?
// What happens if the service goes down ?
// What happens if a node is not in the list anymore ?
// What happens when a new node appears ?
// What happens on first start ?
// How reliable does the service need to be ?

use std::sync::mpsc;

use actix_web::rt::signal;
use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use health_check::HealthCheckLoopConfig;
use notification::NotificationSenderLoopConfig;
use slog::info;

#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_term;

use slog::Drain;
use tokio_util::sync::CancellationToken;

use crate::health_check::start_health_check_loop;
use crate::notification::{start_notification_sender_loop, Sink};
use crate::notification::{LogSink, MatrixSink};

mod config;
mod health_check;
mod matrix;
mod nodes_status;
mod notification;

#[get("/state")]
async fn state() -> impl Responder {
    HttpResponse::Ok().body("Hello !")
}

#[actix_web::main]
async fn main() {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    let log = slog::Logger::root(drain, o!());

    let config = config::Config::new().unwrap();

    let (notif_sender, notif_receiver) = mpsc::channel();
    let cancellation_token = CancellationToken::new();

    actix_web::rt::spawn(start_health_check_loop(HealthCheckLoopConfig {
        logger: log.clone(),
        notification_sender: notif_sender.clone(),
        cancellation_token: cancellation_token.clone(),
    }));

    actix_web::rt::spawn(start_notification_sender_loop(
        NotificationSenderLoopConfig {
            logger: log.clone(),
            notification_receiver: notif_receiver,
            cancellation_token: cancellation_token.clone(),
        },
        vec![
            Sink::Matrix(MatrixSink {
                matrix_client: matrix::Client::from_config(config).await.unwrap(),
                logger: log.clone(),
            }),
            Sink::Log(LogSink { logger: log.clone() }),
        ],
    ));

    info!(log, "Starting server on port 8080");
    let srv = HttpServer::new(|| App::new().service(state))
        .shutdown_timeout(5)
        .disable_signals()
        .bind(("127.0.0.1", 8080))
        .unwrap()
        .run();
    let srv_handle = srv.handle();
    // We need to spawn the server, or we cannot stop it (obviously). This
    // is however not done by the run method, it needs to be spawned on its own.
    // We are not pushing the the same vec as the others since it is a different type.
    // We should not have many tasks, so we can even stop them all manually.
    // We might want to replace those with actors
    actix_web::rt::spawn(srv);

    signal::ctrl_c().await.unwrap();
    debug!(log, "Shutting down threads");
    cancellation_token.cancel();
    debug!(log, "Stopping server");
    srv_handle.stop(true).await
}
