use cell::server::Server;

#[tokio::test]
async fn test_server_new() {
    let (_server, _client) = Server::new();
    // Optionally add assertions on the server or client state
}

#[tokio::test]
async fn test_server_reset() {
    let (mut server, _client) = Server::new();
    server.reset().await;
    // Add assertions here to check if reset modified the server state
}
