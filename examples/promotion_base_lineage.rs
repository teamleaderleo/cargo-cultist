#[allow(dead_code)]
#[path = "../src/promotion_base_lineage.rs"]
mod promotion_base_lineage;
#[allow(dead_code)]
#[path = "../src/promotion_receipt.rs"]
mod promotion_receipt;

use std::error::Error;
use std::io::{self, Read};

use promotion_base_lineage::{PromotionBaseLineageRequest, evaluate_promotion_base_lineage};

fn main() {
    if let Err(error) = run() {
        eprintln!("promotion-base-lineage: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    io::stdin().read_to_end(&mut bytes)?;
    let request: PromotionBaseLineageRequest = serde_json::from_slice(&bytes)?;
    let evaluation = evaluate_promotion_base_lineage(&request)?;
    println!("{}", serde_json::to_string_pretty(&evaluation)?);
    Ok(())
}
