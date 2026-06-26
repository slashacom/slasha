pub fn bytes_to_mib(bytes: u64) -> i32 {
    (bytes / (1024 * 1024)) as i32
}
