//! Measure an indexer-supplied, public signed transaction using Ootle's CBOR codec.
//! Input JSON is downloaded separately from /transactions/{transaction_id}.
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::{env, fs};
use tari_ootle_transaction::Transaction;

fn main() -> Result<()> {
    for path in env::args().skip(1) {
        let value:Value=serde_json::from_slice(&fs::read(&path)?)?;
        let record=&value["transaction"];
        let id=record["transaction_id"].as_str().context("missing indexed transaction ID")?;
        let tx:Transaction=serde_json::from_value(record["transaction"].clone())?;
        ensure!(tx.calculate_id().to_string()==id,"indexed transaction ID mismatch in {path}");
        let bytes=tari_bor::encode(&tx)?.len();
        println!("{id}\t{bytes}\t{path}");
    }
    Ok(())
}
