use std::sync::atomic::{AtomicBool, Ordering};

use actix_web::{get, web, HttpResponse, Responder};

#[derive(Debug)]
pub struct ServiceHealth {
    notification_loop_ready: AtomicBool,
    registry_updater_loop_ready: AtomicBool,
    health_check_loop_ready: AtomicBool,
}

impl ServiceHealth {
    pub fn new() -> Self {
        Self {
            notification_loop_ready: AtomicBool::new(false),
            registry_updater_loop_ready: AtomicBool::new(false),
            health_check_loop_ready: AtomicBool::new(false),
        }
    }

    fn is_ready(&self) -> bool {
        self.health_check_loop_ready.load(Ordering::Relaxed)
            && self.notification_loop_ready.load(Ordering::Relaxed)
            && self.health_check_loop_ready.load(Ordering::Relaxed)
    }

    pub fn set_registry_updater_loop_readiness(&self, status: bool) {
        self.registry_updater_loop_ready.store(status, Ordering::Relaxed)
    }

    pub fn set_notification_loop_readiness(&self, status: bool) {
        self.notification_loop_ready.store(status, Ordering::Relaxed)
    }

    pub fn set_health_check_loop_readiness(&self, status: bool) {
        self.health_check_loop_ready.store(status, Ordering::Relaxed)
    }
}

#[get("/alive")]
pub async fn alive() -> impl Responder {
    HttpResponse::Ok()
}

#[get("/ready")]
pub async fn ready(service_health: web::Data<ServiceHealth>) -> impl Responder {
    if service_health.is_ready() {
        HttpResponse::Ok().body(format!("{:?}", service_health))
    } else {
        HttpResponse::ServiceUnavailable().body(format!("{:?}", service_health))
    }
}
