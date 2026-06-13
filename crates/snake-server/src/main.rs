//! Binary entry point: read config from the environment, open the database
//! and serve until SIGINT/SIGTERM.

use std::net::SocketAddr;

use snake_server::{config::ServerConfig, router, AppState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "snake_server=info,tower_http=info".into()),
        )
        .init();

    let cfg = ServerConfig::from_env();
    let addr: SocketAddr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_owned())
        .parse()?;

    if let Some(parent) = cfg.db_path.as_ref().and_then(|p| p.parent()) {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).ok();
        }
    }

    let state = AppState::new(cfg)?;
    let app = router(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("HexSnake highscore server listening on {addr}");

    // `into_make_service_with_connect_info` exposes the peer address to the
    // rate limiter (used when X-Forwarded-For is not trusted).
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutting down");
}
