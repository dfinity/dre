use axum::Json;

use crate::{handlers::rate::put_rate::put_rate, tests::server};

#[tokio::test]
async fn test_put() {
    let server = server();
    let payload = Json(1200);

    let resp = put_rate(server, payload).await;
    assert!(resp.is_ok());

    let resp = resp.unwrap();
    assert_eq!(resp.0, 1200);
}
