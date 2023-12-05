// All imports for cloudflare specific things
#[cfg(feature = "cloudflare")]
use worker::{event, Context, Date, Env, Request};

// All imports for server specific things
#[cfg(feature = "server")]
use axum::{
    async_trait,
    extract::FromRequest,
    extract::{Path, WebSocketUpgrade},
    http::{Request, StatusCode},
    TypedHeader,
};
#[cfg(feature = "server")]
use axum::{routing::get, Router};
#[cfg(feature = "server")]
use std::collections::HashMap;
#[cfg(feature = "server")]
use std::env;
#[cfg(feature = "server")]
use std::net::SocketAddr;
#[cfg(feature = "server")]
use std::sync::Arc;
#[cfg(feature = "server")]
use tokio::sync::Mutex;
#[cfg(feature = "server")]
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

mod logger;
mod tcp;

#[cfg(feature = "cloudflare")]
mod cloudflare;

#[cfg(feature = "server")]
mod server;

#[cfg(feature = "server")]
struct IdempotencyKey(Option<String>);

#[cfg(feature = "server")]
#[async_trait]
impl<'a, B> FromRequest<(), B> for IdempotencyKey
where
    B: Send + 'static,
{
    type Rejection = StatusCode;

    async fn from_request(req: Request<B>, _: &()) -> Result<Self, Self::Rejection> {
        let headers = req.headers().clone();
        let key = headers
            .get("Idempotency-Key")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        Ok(IdempotencyKey(key))
    }
}

/// Main function for running the program as a server
#[tokio::main]
#[cfg(feature = "server")]
async fn main() {
    println!("Running ln-websocket-proxy");
    tracing_subscriber::fmt::init();

    let locks = Arc::new(Mutex::new(HashMap::new()));

    let app = Router::new()
        .route(
            "/v1/:ip/:port",
            get(
                |path: Path<(String, String)>,
                 ws: WebSocketUpgrade,
                 user_agent: Option<TypedHeader<headers::UserAgent>>,
                 idempotency_key: IdempotencyKey| async move {
                    server::ws_handler(path, ws, user_agent, idempotency_key.0, locks.clone())
                },
            ),
        )
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    let port = match env::var("LN_PROXY_PORT") {
        Ok(p) => p.parse().expect("port must be a u16 string"),
        Err(_) => 3001,
    };
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    println!("Stopping websocket-tcp-proxy");
}

/// Main function for the Cloudflare Worker that triggers off of a HTTP req
#[event(fetch)]
#[cfg(feature = "cloudflare")]
pub async fn main(req: Request, env: Env, _ctx: Context) -> worker::Result<worker::Response> {
    log_request(&req);

    // Optionally, get more helpful error messages written to the console in the case of a panic.
    cloudflare::set_panic_hook();

    // Optionally, use the Router to handle matching endpoints, use ":name" placeholders, or "*name"
    // catch-alls to match on specific patterns. Alternatively, use `Router::with_data(D)` to
    // provide arbitrary data that will be accessible in each route via the `ctx.data()` method.
    let router = worker::Router::new();

    // Add as many routes as your Worker needs! Each route will get a `Request` for handling HTTP
    // functionality and a `RouteContext` which you can use to  and get route parameters and
    // Environment bindings like KV Stores, Durable Objects, Secrets, and Variables.
    router
        .get("/v1/:ip/:port", crate::cloudflare::handle_ws_to_tcp)
        .options("/*catchall", |_, _| crate::cloudflare::empty_response())
        .run(req, env)
        .await
}

#[cfg(feature = "cloudflare")]
fn log_request(req: &Request) {
    crate::logger::info(&format!(
        "Incoming Request: {} - [{}]",
        Date::now().to_string(),
        req.path()
    ));
}
