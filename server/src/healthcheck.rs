//! Local readiness probe, independent of shell, curl, secrets and proxy settings.
pub async fn ready(port: u16) -> bool {
    if port == 0 { return false; }
    let Ok(client) = reqwest::Client::builder().no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(3)).build() else { return false; };
    client.get(format!("http://127.0.0.1:{port}/ready")).send().await
        .is_ok_and(|response| response.status() == reqwest::StatusCode::OK)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    #[tokio::test]
    async fn healthcheck_accepts_ready_and_rejects_failure_or_redirect() {
        for status in [200, 503, 302] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let task = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0; 1024];
                let count = socket.read(&mut request).await.unwrap();
                assert!(String::from_utf8_lossy(&request[..count]).starts_with("GET /ready "));
                socket.write_all(format!("HTTP/1.1 {status} Test\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
            });
            assert_eq!(ready(port).await, status == 200);
            task.await.unwrap();
        }
        assert!(!ready(0).await);
    }
}
