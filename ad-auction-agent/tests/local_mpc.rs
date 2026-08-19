// Integration test: run the sealed-bid auction over local MPC with the
// documented fixture (bids 300, 500, 900, 700). Asserts the privacy-relevant
// properties: the correct winner/price are computed, and each bidder receives
// exactly one output value (its won-bit) -- never another bidder's bid.

use std::path::PathBuf;
use stoffel::prelude::*;

fn auction_source() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/auction.stfl")
}

#[tokio::test]
async fn sealed_bid_auction_reveals_only_winner_and_price() {
    let bids: &[(u64, &[i64])] = &[
        (0, &[300]),
        (1, &[500]),
        (2, &[900]),
        (3, &[700]),
    ];

    let (host_result, client_outputs) = Stoffel::compile_file(auction_source())
        .unwrap()
        .parties(5)
        .threshold(1)
        .expected_output_clients(4)
        .with_client_inputs(bids)
        .execute_local_capturing_client_outputs()
        .await
        .expect("local MPC auction should execute");

    // Host learns winner + price only. main returns list[int64].
    let result = host_result[0].as_list().expect("host result list");
    let winner = result[0].as_i64().expect("winner index");
    let price = result[1].as_i64().expect("winning price");
    assert_eq!(winner, 2, "highest sealed bid (900) is agent slot 2");
    assert_eq!(price, 900, "first-price clearing price equals the winning bid");

    // Every bidder receives exactly ONE value: its won-bit. No bidder sees
    // another bidder's bid or the price.
    assert_eq!(client_outputs.len(), 4, "all four bidders get an output");
    for out in &client_outputs {
        assert_eq!(
            out.values.len(),
            1,
            "bidder slot {} must receive exactly one value (its won-bit)",
            out.client_slot
        );
        let won = out.values[0].as_i64().map(|b| b != 0).unwrap_or(false);
        let expected_won = out.client_slot == 2;
        assert_eq!(
            won, expected_won,
            "only slot 2 should learn it won; slot {} got won={}",
            out.client_slot, won
        );
    }
}
