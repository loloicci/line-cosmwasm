use sha1::{Digest, Sha1};

use crate::errors::CryptoResult;

pub fn sha1_calculate(message: &[u8]) -> CryptoResult<[u8; 20]> {
    let mut hasher = Sha1::new();
    hasher.update(message);
    let buffer: [u8; 20] = hasher.finalize().into();
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex;

    #[test]
    fn test_sha1_calculate() {
        let message = b"The quick brown fox jumps over the lazy dog";
        let result = sha1_calculate(message).unwrap();
        assert_eq!(
            result.to_vec(),
            hex::decode("2fd4e1c67a2d28fced849ee1bb76e7391b93eb12").unwrap()
        )
    }
}
