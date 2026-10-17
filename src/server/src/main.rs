#![recursion_limit = "1024"]

use axum::Router;
use frontend::app::shell;
use leptos::config::get_configuration;
use leptos_axum::generate_route_list_with_ssg;
use tower_http::services::ServeDir;

/// Prerender every route, then optionally serve the generated files for local
/// development. Production nginx serves the files directly.
#[tokio::main]
async fn main() {
    let conf = get_configuration(None).expect("Failed to read Leptos config");
    let options = conf.leptos_options;
    let addr = options.site_addr;
    let site_root = options.site_root.to_string();

    let (_routes, generator) = generate_route_list_with_ssg({
        let options = options.clone();
        move || shell(options.clone())
    });
    generator.generate(&options).await;

    if std::env::args().any(|arg| arg == "--generate-only") {
        return;
    }

    let app = Router::new()
        .fallback_service(ServeDir::new(site_root).append_index_html_on_directories(true));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
