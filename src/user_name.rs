use crate::{SdkError, SdkResult};

pub const MAX_NAME_LENGTH: usize = 32;
pub const DEFAULT_USER_NAME: &str = "Main Account";
pub const DEFAULT_MARKET_NAME: &str = "Default Market Name";

pub fn encode_name(name: &str) -> SdkResult<[u8; 32]> {
    if name.len() > MAX_NAME_LENGTH {
        let err = format!("Name ({name}) longer than 32 characters");
        return Err(SdkError::Generic(err));
    }

    let mut buffer = [0u8; 32];
    let bytes_to_copy = name.as_bytes().len().min(buffer.len());
    buffer[..bytes_to_copy].copy_from_slice(&name.as_bytes()[..bytes_to_copy]);

    for i in bytes_to_copy..buffer.len() {
        buffer[i] = b' ';
    }

    Ok(buffer)
}
