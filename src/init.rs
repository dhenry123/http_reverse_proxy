use tracing_subscriber::EnvFilter;

pub fn init_logging() {
    // // Configure async JSON logging in production
    // #[cfg(not(debug_assertions))]
    // {
    //     use tracing_subscriber::fmt::writer::MakeWriterExt;
    //     let file_appender = tracing_appender::rolling::daily("/var/log/myapp", "tls-server.log");
    //     let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    //     tracing_subscriber::fmt()
    //         .json()
    //         .with_writer(non_blocking)
    //         .with_env_filter(EnvFilter::new("info,tls=debug"))
    //         .init();
    // }

    // // More verbose logging for development
    // #[cfg(debug_assertions)]
    // {
    tracing_subscriber::fmt()
        .pretty()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true) // false for production
        .init();
    // }
}
