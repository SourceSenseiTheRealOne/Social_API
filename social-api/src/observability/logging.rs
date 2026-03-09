use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_logging() {
    tracing_subscriber::registry()
        .with(
            EnvFilter::from_default_env()
                .add_directive("social_api=info".parse().unwrap())
                .add_directive("sqlx=warn".parse().unwrap())
                .add_directive("tower_http=info".parse().unwrap()),
        )
        .with(
            fmt::layer()
                .json()
                .with_target(true)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false)
                .flatten_event(true),
        )
        .init();
}
