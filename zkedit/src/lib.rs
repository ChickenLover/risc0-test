//#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, Deserialize, Serialize, PartialEq, Hash)]
pub struct ImageData {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<Vec<u32>>,
}