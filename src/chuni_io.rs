use log::info;

type HRESULT = i32;

#[unsafe(no_mangle)]
pub extern "C" fn chuni_io_get_api_version() -> u16 {
    info!("chuni_io_get_api_version called");
    0x0102
}

mod jvs;
mod led;
mod slider;
