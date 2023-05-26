pub(crate) fn info(message: &str) {
    #[cfg(feature = "cloudflare")]
    worker::console_log!("INFO: {}", message);

    #[cfg(feature = "server")]
    tracing::info!("{}", message);
}

pub(crate) fn debug(message: &str) {
    #[cfg(feature = "cloudflare")]
    worker::console_log!("DEBUG: {}", message);

    #[cfg(feature = "server")]
    tracing::debug!("{}", message);
}

pub(crate) fn error(message: &str) {
    #[cfg(feature = "cloudflare")]
    worker::console_log!("ERROR: {}", message);

    #[cfg(feature = "server")]
    tracing::error!("{}", message);
}
