use crate::{logger, tcp::connect_to_addr};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, TypedHeader,
    },
    response::IntoResponse,
};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

pub(crate) async fn ws_handler(
    Path((ip, port)): Path<(String, String)>,
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
) -> impl IntoResponse {
    logger::info(&format!("ip: {ip}, port: {port}"));
    if let Some(TypedHeader(user_agent)) = user_agent {
        logger::info(&format!("`{user_agent}` connected"));
    }

    ws.protocols(["binary"])
        .on_upgrade(move |socket| handle_socket(socket, ip, port))
}

pub(crate) async fn handle_socket(mut socket: WebSocket, host: String, port: String) {
    let server_stream = match connect_to_addr(host, port, server_tcp_connect) {
        Ok(s) => s,
        Err(e) => {
            logger::error(&format!("Error connecting to address: {e:?}"));
            return;
        }
    };
    if let Err(e) = handle_server_stream(socket, server_stream).await {
        logger::error(&format!("Error handling server stream: {e:?}"));
    }
}

fn server_tcp_connect(addr: std::net::SocketAddr) -> Option<TcpStream> {
    match std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(10)) {
        Ok(conn) => {
            conn.set_nonblocking(true).unwrap();
            let connection = TcpStream::from_std(conn);
            if let Err(error) = &connection {
                logger::error(&format!("Could not connect to {addr}: {error}"));
            } else {
                logger::debug(&format!("Connected to destination server: {addr}"));
            }

            connection.ok()
        }
        Err(_) => None,
    }
}

async fn handle_server_stream(
    mut socket: WebSocket,
    mut server_stream: TcpStream,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = server_stream.peer_addr().unwrap();
    logger::info(&format!("Handling server stream for: {addr}"));

    let mut buf = [0u8; 65536]; // the max lightning message size is 65536

    loop {
        tokio::select! {
            res  = socket.recv() => {
                if let Some(msg) = res {
                    if let Ok(Message::Binary(msg)) = msg {
                        logger::debug(&format!("Received {}, sending to {addr}", &msg.len()));
                        let _ = server_stream.write_all(&msg).await;
                    }
                } else {
                    logger::info("Client close");
                    break;
                }
            },
            res  = server_stream.read(&mut buf) => {
                match res {
                    Ok(n) => {
                        logger::debug(&format!("Read {n} bytes from server: {addr}"));
                        if 0 != n {
                            let _ = socket.send(Message::Binary(buf[..n].to_vec())).await;
                        } else {
                            break;
                        }
                    },
                    Err(e) => {
                        logger::info(&format!("Server {addr} close with error: {e:?}"));
                        break;
                    }
                }
            },
        }
    }

    Ok(())
}
