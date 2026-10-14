use rand::distr::{Alphanumeric, SampleString};
use rand::rng;
use std::path::Path;

pub fn generate_random_string(length: usize, to_lowercase: bool) -> String {
    let value = Alphanumeric.sample_string(&mut rng(), length);
    if to_lowercase {
        value.to_lowercase()
    } else {
        value
    }
}

pub async fn delete_file<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    tokio::fs::remove_file(path).await
}
