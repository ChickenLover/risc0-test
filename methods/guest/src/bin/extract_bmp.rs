#![no_main]
#![no_std]

use risc0_zkvm_guest::{env, sha};

extern crate alloc;
use alloc::vec::Vec;

use bmp::{Image};
use zkedit::ImageData;

risc0_zkvm_guest::entry!(main);

pub fn main() {
    let file_bytes: Vec<u8> = env::read();
    env::commit(&sha::digest(&file_bytes));

    /*
    let image: Image = bmp::from_bytes(&file_bytes).unwrap();
    let mut data: ImageData = ImageData {
        width: image.get_width(),
        height: image.get_height(),
        pixels: Vec::new()
    };

    /*
    for _ in 0..data.height {
        let mut row: Vec<u32> = Vec::new();
        for x in 0..data.width {
            row.push(x);
        }
        data.pixels.push(row);
    }
    */
    env::commit(&sha::digest(&data));
    */
}
