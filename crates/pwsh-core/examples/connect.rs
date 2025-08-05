fn main() {
    tracing_subscriber::fmt::SubscriberBuilder::default()
        .with_target(false)
        .with_line_number(true)
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let host = "10.10.0.3";



}
