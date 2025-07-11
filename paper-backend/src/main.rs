mod app_data;
mod config;
mod error;
mod model;
mod router;
// mod timed_task;
mod utils;

use salvo::{
    http::Method,
    oapi::{
        SecurityScheme,
        security::{Http, HttpAuthScheme},
    },
    prelude::*,
};
// use timed_task::register_timed_task;
use tracing::info;

use crate::utils::jwt::set_jwt_config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = std::env::args().collect::<Vec<_>>();

    let config = config::Config::from_path(opt.get(1).unwrap_or(&"config.toml".into()))
        .expect("Failed to load config");
    let _g = ai_flow_synth::utils::enable_log(&config.log_config).unwrap();
    set_jwt_config(&config.backend_config.jwt);
    let app_data = app_data::AppData::new(&config).await;

    // register_timed_task(app_data.clone()).await;

    let cors = salvo::cors::Cors::new()
        .allow_origin(
            config
                .frontend_config
                .cors
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        )
        .allow_methods(vec![Method::GET, Method::POST, Method::DELETE, Method::PUT])
        .allow_headers(vec!["authorization", "content-type"])
        .into_handler();

    let router = Router::new().push(
        Router::with_path("api")
            .hoop(affix_state::inject(app_data))
            .push(router::create_router(&config.backend_config)),
    );
    let doc = OpenApi::new("Paper Api", "0.0.1")
        .add_security_scheme(
            "bearer",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer).bearer_format("JWT")),
        )
        .merge_router(&router);

    let router = router
        .unshift(doc.into_router("/api-doc/openapi.json"))
        .unshift(SwaggerUi::new("/api-doc/openapi.json").into_router("/swagger-ui"));

    let service = Service::new(router).hoop(cors);

    let acceptor = TcpListener::new(&config.backend_config.address)
        .bind()
        .await;
    Server::new(acceptor).serve(service).await;
    info!("Server started on {}", &config.backend_config.address);

    Ok(())
}
