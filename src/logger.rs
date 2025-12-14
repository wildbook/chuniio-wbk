use std::fs::OpenOptions;

use tracing_subscriber::{Layer, Registry, filter::LevelFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

pub type Guard = ();

pub fn init_logger() -> Guard {
    let level = match cfg!(debug_assertions) {
        true => LevelFilter::TRACE,
        false => LevelFilter::INFO,
    };

    let suffix = match std::mem::size_of::<usize>() {
        4 => "x86",
        8 => "x64",
        _ => unreachable!(),
    };

    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(format!("log-chuniio-{suffix}.log"))
        .unwrap();

    Registry::default()
        .with(
            fmt::layer() //
                .with_target(true)
                .with_level(true)
                .without_time()
                .compact()
                .with_filter(level),
        )
        .with(
            fmt::layer() //
                .with_ansi(false)
                .with_target(true)
                .with_level(true)
                .compact()
                .with_writer(file),
        )
        .init();

    tracing::info!("logger initialized!");
}
