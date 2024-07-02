use axum::Json;

use crate::{
    handlers::criteria::{delete_criteria::delete_criteria, get_criteria::get_criteria, post_criteria::update},
    tests::{server, server_with_criteria},
};

#[tokio::test]
async fn delete_criteria_test() {
    let server = server_with_criteria(["test", "another", "one more"].iter().map(|f| f.to_string()).collect());
    let payload = Json(vec![2, 0]);

    let resp = delete_criteria(server, payload).await;

    assert!(resp.is_ok());
    let resp = resp.unwrap();
    assert!(!resp.is_empty());
    assert!(resp.first_key_value().is_some());
    let (key, value) = resp.first_key_value().unwrap();
    assert_eq!(key, &0);
    assert_eq!(value, "another")
}

#[tokio::test]
async fn delete_criteria_error_test() {
    let server = server();
    let payload = Json(vec![1]);

    let resp = delete_criteria(server, payload).await;

    assert!(resp.is_err());
}

#[tokio::test]
async fn get_criteria_test() {
    let posted_criteria: Vec<String> = ["test", "another", "one more"].iter().map(|f| f.to_string()).collect();
    let server = server_with_criteria(posted_criteria.clone());
    let resp = get_criteria(server).await;

    assert!(resp.is_ok());
    let resp = resp.unwrap();
    assert_eq!(resp.len(), posted_criteria.len());
    for i in 0..posted_criteria.len() {
        assert_eq!(resp[&(i as u32)], posted_criteria[i]);
    }
}

#[tokio::test]
async fn post_criteria_test() {
    let server = server_with_criteria(vec![]);
    let payload = Json(["test", "another", "one more"].iter().map(|f| f.to_string()).collect::<Vec<String>>());

    let resp = update(server, payload).await;

    assert!(resp.is_ok());
    let resp = resp.unwrap();
    assert!(!resp.is_empty());
    assert!(resp.first_key_value().is_some());
    let (key, value) = resp.first_key_value().unwrap();
    assert_eq!(key, &0);
    assert_eq!(value, "test")
}
