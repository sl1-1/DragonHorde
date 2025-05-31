use image::{DynamicImage};
use img_hash::HashAlg::Gradient;
use img_hash::{HasherConfig};
use sha2::{Digest, Sha256};
use crate::error::AppError;

pub fn perceptual(im: &DynamicImage) -> i64 {
    let image_hash = HasherConfig::with_bytes_type::<[u8; 8]>()
        .hash_alg(Gradient)
        .hash_size(8, 8)
        .preproc_dct()
        .to_hasher()
        .hash_image(im);
    let hash: [u8; 8] = image_hash.as_bytes().try_into().expect("Couldn't convert to bytes");
    i64::from_be_bytes(hash)
}

pub fn sha256(data: &[u8]) -> String {
    //Hash
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();
    format!("{:x}", hash)
}
