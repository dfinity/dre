use serde::de::DeserializeOwned;

pub async fn query_ic_dashboard_list<T: DeserializeOwned>(path: &str) -> anyhow::Result<T> {
    let client = reqwest::Client::new();
    let result = client
        .get(format!("https://ic-api.internetcomputer.org/api/{}", &path))
        .send()
        .await
        .and_then(|r| r.error_for_status());
    match result {
        Ok(response) => match response.json::<T>().await {
            Ok(data) => Ok(data),
            Err(e) => Err(anyhow::format_err!("failed to parse response: {}", e)),
        },
        Err(e) => Err(anyhow::format_err!("failed to fetch response: {}", e)),
    }
}
