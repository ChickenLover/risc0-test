#![no_main]
#![no_std]

use risc0_zkvm_guest::{env, sha};

risc0_zkvm_guest::entry!(main);

pub fn main() {
    let file_bytes = env::read();
    

    env::commit(&sha::digest(&state));
}
