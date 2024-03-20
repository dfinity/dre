// The goal is to be able to notify Node Providers of changes in status of their
// nodes. Here are some elements that we might want to include
//
// - Fetch data from prometheus
// - Refresh state based on this data, maybe with a log of state changes
// - Send message on status change
// - Fetch latest known state on startup, to notify in case of a change during a
//   restart
//
// We should also have an API to get the current status
// GET /api/v1/state/<node-id>
// GET /api/v1/state
//
// We should have a webhook API
// We want a way to register new webhooks
//
// We want to have a way to send random events in the service, and see them go
// through. Can be a webhook, a simple log sink.
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
use actix_web::{web, App, HttpServer};
use health_check::HealthCheckLoopConfig;

use notification::NotificationSenderLoopConfig;

use tokio_util::sync::CancellationToken;
use tracing::{debug, info, Level};
use tracing_log::LogTracer;
use tracing_subscriber::FmtSubscriber;

use crate::health_check::start_health_check_loop;
use crate::notification::start_notification_sender_loop;
use crate::registry::{start_registry_updater_loop, RegistryLoopConfig};
use crate::router::Router;
use crate::service_health::ServiceHealth;
use crate::sink::{LogSink, Sink};

mod health_check;
mod nodes_status;
mod notification;
mod registry;
mod router;
mod service_health;
mod sink;

#[actix_web::main]
async fn main() {
    let subscriber = FmtSubscriber::builder().with_max_level(Level::INFO).compact().finish();
    LogTracer::builder().init().expect("can create a log tracer");
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // TODO Centralize sending all notifications using the router
    let router = Router::new_from_config_file().expect("should create a new router");

    let (notif_sender, notif_receiver) = mpsc::channel();
    let cancellation_token = CancellationToken::new();

    let service_health = web::Data::new(ServiceHealth::new());

    actix_web::rt::spawn(start_registry_updater_loop(RegistryLoopConfig {
        cancellation_token: cancellation_token.clone(),
        target_network: target_network(),
        service_health: service_health.clone(),
    }));

    actix_web::rt::spawn(start_health_check_loop(HealthCheckLoopConfig {
        notification_sender: notif_sender.clone(),
        cancellation_token: cancellation_token.clone(),
        registry_state: registry::create_registry_state().await,
        service_health: service_health.clone(),
    }));

    actix_web::rt::spawn(start_notification_sender_loop(
        NotificationSenderLoopConfig {
            notification_receiver: notif_receiver,
            cancellation_token: cancellation_token.clone(),
            router,
            service_health: service_health.clone(),
        },
        vec![Sink::Log(LogSink {})],
    ));

    info!("Starting server on port 8080");
    let srv = HttpServer::new(move || {
        App::new()
            .app_data(service_health.clone())
            .service(service_health::alive)
            .service(service_health::ready)
    })
    .shutdown_timeout(5)
    .disable_signals()
    .bind(("127.0.0.1", 8080))
    .unwrap()
    .run();
    let srv_handle = srv.handle();
    // We need to spawn the server, or we cannot stop it (obviously). This
    // is however not done by the run method, it needs to be spawned on its own.
    // We are not pushing the the same vec as the others since it is a different
    // type. We should not have many tasks, so we can even stop them all
    // manually. We might want to replace those with actors
    actix_web::rt::spawn(srv);

    // TODO If the token is cancelled by any other way, the service will not
    // completely shut down.  Review all the cases, so that the service is more
    // resilient
    signal::ctrl_c().await.unwrap();
    debug!("Shutting down threads");
    cancellation_token.cancel();
    debug!("Stopping server");
    srv_handle.stop(true).await
}
