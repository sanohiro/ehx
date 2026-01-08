mod document;

pub use document::Document;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum BufferError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Position out of bounds: {0}")]
    OutOfBounds(usize),
}
