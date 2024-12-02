pub fn init_sentry() -> sentry::ClientInitGuard {
    let mut options = sentry::ClientOptions::new();
    options.release = sentry::release_name!();
    options.attach_stacktrace = true;

    let guard = sentry::init((
        "https://fd712925b5fc9c2bc1ac4edf3d1c0b82@sentry.wposek.ru/5",
        options,
    ));

    guard
}
