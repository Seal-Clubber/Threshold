//! Native proof microbenchmark. This does not execute Ootle consensus or include transaction framing.
use std::time::Instant;
use anyhow::Result;
use threshold_client::{demo_secret, escrow_input, private_output, private_transfer};
use tari_engine_types::stealth::validate_transfer;
use tari_ootle_wallet_crypto::MaskAndValue;
use tari_template_lib::prelude::ComponentAddress;
use tari_template_lib::types::ObjectKey;

fn main() -> Result<()> {
    let component = ComponentAddress::new(ObjectKey::from_array([7; 32]));
    let recipient = demo_secret().to_address();
    println!("inputs,proof_ms_median_5,statement_cbor_bytes,validation_ms_median_5");
    for count in [1usize, 4, 16, 32] {
        let cells = (0..count)
            .map(|_| private_output(&recipient, 1_000_000, Some(component)))
            .collect::<Result<Vec<_>>>()?;
        let inputs = cells.into_iter()
            .map(|cell| escrow_input(MaskAndValue { mask: cell.witness.mask, value: cell.witness.amount }, component))
            .collect::<Result<Vec<_>>>()?;
        let output = private_output(&recipient, count as u64 * 1_000_000, None)?;
        let mut proof_ms = Vec::new();
        let mut validation_ms = Vec::new();
        let mut bytes = 0;
        for _ in 0..5 {
            let start = Instant::now();
            let statement = private_transfer(inputs.clone(), std::slice::from_ref(&output))?;
            proof_ms.push(start.elapsed().as_millis());
            bytes = tari_bor::encode(&statement)?.len();
            let start = Instant::now();
            validate_transfer(&statement, None)?;
            validation_ms.push(start.elapsed().as_millis());
        }
        proof_ms.sort_unstable();
        validation_ms.sort_unstable();
        println!("{count},{},{bytes},{}", proof_ms[2], validation_ms[2]);
    }
    Ok(())
}
