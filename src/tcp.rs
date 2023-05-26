use std::net::ToSocketAddrs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub fn connect_to_addr<S, F>(host: String, port: String, connect_fn: F) -> Result<S, &'static str>
where
    S: AsyncReadExt + AsyncWriteExt,
    F: Fn(std::net::SocketAddr) -> Option<S>,
{
    let addr_str = format_addr_from_url(host, port);
    let addr_lookup = addr_str.to_socket_addrs();

    if addr_lookup.is_err() || addr_lookup.as_ref().unwrap().len() == 0 {
        return Err("Could not resolve address");
    }

    let mut addrs = addr_lookup.unwrap();
    let server_stream = addrs.find_map(connect_fn);
    if server_stream.is_none() {
        return Err("Could not connect to address");
    }

    Ok(server_stream.unwrap())
}

pub(crate) fn format_addr_from_url(ip: String, port: String) -> String {
    format!("{}:{}", ip.replace('_', "."), port)
}

#[cfg(test)]
mod tests {
    use crate::tcp::format_addr_from_url;

    #[tokio::test]
    async fn test_format_addr_from_url() {
        assert_eq!(
            "127.0.0.1:9000",
            format_addr_from_url(String::from("127_0_0_1"), String::from("9000"))
        )
    }
}
