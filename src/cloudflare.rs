use crate::{logger, tcp::connect_to_addr};
use cfg_if::cfg_if;
use futures::StreamExt;
use futures::{future::Either, pin_mut, FutureExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use worker::*;

pub(crate) fn handle_ws_to_tcp(_req: Request, ctx: RouteContext<()>) -> worker::Result<Response> {
    if ctx.param("ip").is_none() {
        return Response::error("IP missing", 400);
    }
    if ctx.param("port").is_none() {
        return Response::error("IP missing", 400);
    }
    let host = ctx.param("ip").unwrap().to_string();
    let port = ctx.param("port").unwrap().to_string();

    let server_stream = connect_to_addr(host.clone(), port, worker_tcp_connect)?;

    // For websocket compatibility
    let pair = WebSocketPair::new()?;
    let server = pair.server;
    server.accept()?;
    logger::debug("accepted websocket, about to spawn event stream");
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(e) = handle_server_stream(server, server_stream, host).await {
            logger::error(&format!("Error handling server stream: {e:?}"));
        }
    });
    Response::from_websocket(pair.client)
}

fn worker_tcp_connect(addr: std::net::SocketAddr) -> Option<Socket> {
    match ConnectionBuilder::new()
        .secure_transport(SecureTransport::Off)
        .connect(addr.ip().to_string(), addr.port())
    {
        Ok(s) => Some(s),
        Err(e) => {
            logger::error(&format!("Could not connect to {}: {e:?}", addr.ip()));
            None
        }
    }
}

async fn handle_server_stream(
    socket: worker::WebSocket,
    mut server_stream: Socket,
    addr: String,
) -> worker::Result<worker::Response> {
    logger::debug(&format!("Handling server stream for: {addr}"));

    let mut buf = [0u8; 65536]; // the max lightning message size is 65536

    let mut socket_stream = socket.events().expect("could not open stream");

    loop {
        let socket_next = socket_stream.next().fuse();
        let server_read = server_stream.read(&mut buf).fuse();

        pin_mut!(socket_next, server_read);

        match futures::future::select(socket_next, server_read).await {
            Either::Left((msg_res, _)) => {
                if let Some(msg) = msg_res {
                    match msg {
                        Ok(msg) => match msg {
                            WebsocketEvent::Message(msg) => {
                                let b = msg.bytes();
                                if b.is_none() {
                                    continue;
                                }
                                logger::debug(&format!("Received ws data, sending to {addr}"));
                                let _ = server_stream.write_all(&b.unwrap()).await;
                            }
                            WebsocketEvent::Close(event) => {
                                logger::debug(&format!("Closed! {event:?}"))
                            }
                        },
                        Err(e) => {
                            logger::error(&format!("Error reading msg: {e:?}"));
                            break;
                        }
                    }
                } else {
                    logger::debug("Client close");
                    break;
                }
            }
            Either::Right((res, _)) => match res {
                Ok(n) => {
                    logger::debug(&format!("Read {n} bytes from server: {addr}"));
                    if 0 != n {
                        let _ = socket.send_with_bytes(&buf[..n]);
                    } else {
                        break;
                    }
                }
                Err(e) => {
                    logger::error(&format!("Server {addr} close with error: {e:?}"));
                    break;
                }
            },
        }
    }

    empty_response()
}

// TODO
pub(crate) fn handle_mutiny_to_mutiny(
    _req: Request,
    _ctx: RouteContext<()>,
) -> worker::Result<Response> {
    // For websocket compatibility
    let pair = WebSocketPair::new()?;
    let server = pair.server;
    server.accept()?;
    logger::debug("accepted websocket, about to spawn event stream");
    wasm_bindgen_futures::spawn_local(async move {
        let mut event_stream = server.events().expect("stream error");
        logger::debug("spawned event stream, waiting for first message..");
        while let Some(event) = event_stream.next().await {
            if let Err(e) = event {
                logger::error(&format!("error parsing some event: {e}"));
                continue;
            }
            match event.expect("received error in websocket") {
                WebsocketEvent::Message(msg) => {
                    if msg.text().is_none() {
                        continue;
                    };
                    // TODO parse msg when we support m2m here
                }
                WebsocketEvent::Close(_) => {
                    logger::debug("closing");
                    break;
                }
            }
        }
    });
    Response::from_websocket(pair.client)
}

pub(crate) fn empty_response() -> worker::Result<Response> {
    Response::empty()?.with_cors(&cors())
}

pub(crate) fn cors() -> Cors {
    Cors::new()
        .with_credentials(true)
        .with_origins(vec!["*"])
        .with_allowed_headers(vec!["Content-Type"])
        .with_methods(Method::all())
}

pub enum Error {
    /// Worker error
    WorkerError(String),
}

impl From<worker::Error> for Error {
    fn from(e: worker::Error) -> Self {
        Error::WorkerError(e.to_string())
    }
}

cfg_if! {
    // https://github.com/rustwasm/console_error_panic_hook#readme
    if #[cfg(feature = "console_error_panic_hook")] {
        extern crate console_error_panic_hook;
        pub use self::console_error_panic_hook::set_once as set_panic_hook;
    } else {
        #[inline]
        pub fn set_panic_hook() {}
    }
}
