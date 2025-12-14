use cute_log::Logger;

pub fn init_logger() {
    const LOGGER: Logger = Logger::new();
    LOGGER.set_max_level(::log::LevelFilter::Info);
    LOGGER.set_logger().expect("Failed to set logger");

    log::info!("logger initialized!");
}
