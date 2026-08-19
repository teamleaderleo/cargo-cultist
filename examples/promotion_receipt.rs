#[allow(dead_code)]
#[path = "../src/promotion_receipt.rs"]
mod promotion_receipt;

use std::error::Error;
use std::io::{self, Read};

use promotion_receipt::{evaluate_promotion_receipt, parse_promotion_receipt_request};

fn main() {
    if let Err(error) = run() {
        eprintln!("promotion-receipt: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    io::stdin().read_to_end(&mut bytes)?;
    let request = parse_promotion_receipt_request(&bytes)?;
    let evaluation = evaluate_promotion_receipt(&request)?;
    println!("{}", serde_json::to_string_pretty(&evaluation)?);
    Ok(())
}
