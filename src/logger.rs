use std::io::Write;

pub fn init_logger() {
    env_logger::builder()
        .filter_level(::log::LevelFilter::Trace)
        .parse_default_env()
        .format(|f, record| {
            let target = record.target();
            let level = record.level();

            writeln!(f, "{level} {target} -> {}", record.args())
        })
        .init();
}
