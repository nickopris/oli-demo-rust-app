use oli_demo_rust_app::web::{router, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("oli-demo-rust-app listening on {addr}");
    axum::serve(listener, router(AppState)).await?;
    Ok(())
}
