use std::path::PathBuf;

use zkedit::{ImageData};

use methods::{EXTRACT_BMP_ID, EXTRACT_BMP_PATH};
use risc0_zkvm_host::Prover;
// use risc0_zkvm::serde::{from_slice, to_vec};

extern crate byteorder;

use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};

#[test]
fn run() {
    // Make the prover.
    let method_code = std::fs::read(EXTRACT_BMP_PATH).unwrap();
    let mut prover = Prover::new(&method_code, EXTRACT_BMP_ID).unwrap();
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("tests");
    d.push("img_orig.bmp");
    
    let file_bytes = std::fs::read(d).unwrap();
    let mut u32_to_send: Vec<u32> = vec![];

    for chunk in file_bytes.as_slice().chunks(4) {
        let mut vec = [0u8; 4];
        vec[..chunk.len()].copy_from_slice(chunk);
        u32_to_send.push(vec.as_slice().read_u32::<BigEndian>().unwrap());
    }

    prover.add_input(&u32_to_send.as_slice()).unwrap();

    // Run prover & generate receipt
    let receipt = prover.run()
        .expect("Valid code should be provable if it doesn't overflow the cycle limit. See `embed_methods_with_options` for information on adjusting maximum cycle count.");

    // Optional: Verify receipt to confirm that recipients will also be able to verify your receipt
    receipt.verify(EXTRACT_BMP_ID)
        .expect("Code you have proven should successfully verify; did you specify the correct method ID?");
}