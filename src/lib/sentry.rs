pub fn init_sentry() -> sentry::ClientInitGuard {
    let mut options = sentry::ClientOptions::new();
    options.release = sentry::release_name!();
    options.attach_stacktrace = true;

    let guard = sentry::init((
        "https://a618f8bb37c44dd7a8c3a17963aa28fb@o1006895.ingest.sentry.io/6001259",
        options,
    ));

    guard
}
