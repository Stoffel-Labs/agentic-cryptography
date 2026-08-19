use stoffel::prelude::*;

// Typed client IO bindings generated from the auction bytecode at build time.
#[allow(dead_code, unused_mut, unused_variables)]
mod stoffel_bindings {
    include!(concat!(env!("OUT_DIR"), "/stoffel_bindings.rs"));
}

// Ad-Space Auction for AI Agents -- participant/auctioneer harness.
//
// Four AI agents each seal a single bid. The plaintext bid never leaves the
// bidder's own process; it is submitted as a secret share to the MPC service.
// The MPC computes the winner (argmax) and the clearing price without opening
// any bid. At the very end of the auction, the ONLY things revealed are:
//   * to each bidder   -> a single bit: did I win?
//   * to the auctioneer -> the winner index and the winning price
// No bidder learns another bidder's value, and the auctioneer never sees the
// losing bids.
#[tokio::main]
async fn main() -> stoffel::Result<()> {
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/auction.stfl");

    // In production each (slot, bid) pair is submitted by a separate
    // participant-owned client process. Here we submit them through one trusted
    // local harness to exercise the program semantics.
    let bids: &[(u64, &[i64])] = &[
        (0, &[300]),
        (1, &[500]),
        (2, &[900]),
        (3, &[700]),
    ];

    let (host_result, client_outputs) = Stoffel::compile_file(&source)?
        .manifest::<stoffel_bindings::ProgramManifest>()
        .parties(5)
        .threshold(1)
        .expected_output_clients(4)
        .with_client_inputs(bids)
        .execute_local_capturing_client_outputs()
        .await?;

    // Auctioneer (host) learns ONLY the winner and the winning price.
    // main returns list[int64]; inspect the actual SDK shape once.
    eprintln!("DEBUG host_result len = {}", host_result.len());
    eprintln!("DEBUG host_result = {:?}", host_result);
    let result = host_result[0].as_list().expect("host result list");
    let winner = result[0].as_i64().expect("winner index");
    let price = result[1].as_i64().expect("winning price");
    println!("=== Auctioneer (host) sees ===");
    println!("  winning agent slot : {winner}");
    println!("  winning price      : {price}");
    println!("  (every losing bid stays secret -- the host never sees them)");

    // Each bidding agent learns ONLY whether it won.
    println!("=== Each bidding agent learns (and nothing else) ===");
    for out in &client_outputs {
        let won = out
            .values
            .first()
            .and_then(|v| v.as_i64())
            .map(|b| b != 0)
            .unwrap_or(false);
        println!("  agent slot {} : won = {won}", out.client_slot);
    }

    Ok(())
}
