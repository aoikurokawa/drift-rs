use crate::{SdkError, SdkResult};

pub const MAX_NAME_LENGTH: usize = 32;
pub const DEFAULT_USER_NAME: &str = "Main Account";
pub const DEFAULT_MARKET_NAME: &str = "Default Market Name";

pub fn encode_name(name: &str) -> SdkResult<Vec<char>> {
    if name.len() > MAX_NAME_LENGTH {
        let err = format!("Name ({name}) longer than 32 characters");
        return Err(SdkError::Generic(err));
    }

    let mut buffer: Vec<char> = Vec::with_capacity(32);
    for ch in name.chars() {
        buffer.push(ch);
    }

    for _ in 0..name.len() {
        buffer.push(' ');
    }
    // buffer[..bytes_to_copy].copy_from_slice(&name.chars()[..bytes_to_copy]);

    // for i in bytes_to_copy..buffer.len() {
    //     buffer[i] = ' ';
    // }

    Ok(buffer[..buffer.len()].to_vec())
}
