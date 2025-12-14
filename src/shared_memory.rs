use std::fmt::Display;

use anyhow::Context;
pub use shared_memory::Shmem;
use shared_memory::{ShmemConf, ShmemError};

pub fn create<S>(os_id: S, size: usize) -> anyhow::Result<Shmem>
where
    S: AsRef<str> + Display + Copy,
{
    let shmem_conf = ShmemConf::new()
        .size(size) //
        .os_id(os_id);

    let shmem_result = match shmem_conf.clone().create() {
        Err(ShmemError::MappingIdExists) => shmem_conf.open(),
        e => e,
    };

    shmem_result.with_context(|| format!("Failed to create/open shared memory {os_id}"))
}
