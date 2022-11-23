use methods::{EXTRACT_ID, EXTRACT_PATH};
use risc0_zkvm::host::Prover;
// use risc0_zkvm::serde::{from_slice, to_vec};

fn main() {
    // Make the prover.
    let method_code = std::fs::read(EXTRACT_PATH)
        .expect("Method code should be present at the specified path; did you use the correct *_PATH constant?");
    let mut prover = Prover::new(&method_code, EXTRACT_ID)
        .expect("Prover should be constructed from valid method source code and corresponding method ID");

    let file_bytes = std::fs::read("img_orig.bmp").unwrap();

    prover.add_input(file_bytes.as_slice()).unwrap();

    // Run prover & generate receipt
    let receipt = prover.run()
        .expect("Valid code should be provable if it doesn't overflow the cycle limit. See `embed_methods_with_options` for information on adjusting maximum cycle count.");

    // Optional: Verify receipt to confirm that recipients will also be able to verify your receipt
    receipt.verify(EXTRACT_ID)
        .expect("Code you have proven should successfully verify; did you specify the correct method ID?");

    // TODO: Implement code for transmitting or serializing the receipt for other parties to verify here
}
