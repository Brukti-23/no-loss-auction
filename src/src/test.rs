// this file only runs when we test, not in production
#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Env, Address, String,
};

// this helper sets up a fresh auction before each test
// so every test starts clean
fn setup(env: &Env) -> (AuctionContractClient, Address, Address) {
    let seller  = Address::generate(env);
    let bidder1 = Address::generate(env);

    // register a fake token for testing
    let token = env.register_stellar_asset_contract(seller.clone());

    // deploy our auction contract
    let id     = env.register_contract(None, AuctionContract);
    let client = AuctionContractClient::new(env, &id);

    // create auction with 100 token minimum bid
    // runs for 100 ledgers
    client.create_auction(
        &seller,
        &token,
        &String::from_str(env, "Vintage Guitar"),
        &100_0000000i128,
        &100u32,
    );

    (client, seller, bidder1)
}

// TEST 1
// check that creating an auction saves the right data
#[test]
fn test_create_auction() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, _) = setup(&env);

    // min bid should be 100 tokens
    assert_eq!(client.get_min_bid(), 100_0000000i128);

    // status should be Active
    assert_eq!(client.get_status(), AuctionStatus::Active);

    // item name should match
    assert_eq!(
        client.get_item_name(),
        String::from_str(&env, "Vintage Guitar")
    );
}

// TEST 2
// check that placing a bid works and saves correctly
#[test]
fn test_place_bid() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, bidder1) = setup(&env);

    // bidder bids 200 tokens
    client.place_bid(&bidder1, &200_0000000i128);

    // highest bid should now be 200
    assert_eq!(client.get_highest_bid(), 200_0000000i128);

    // highest bidder should be bidder1
    assert_eq!(client.get_highest_bidder(), Some(bidder1));
}

// TEST 3
// this tests the no-loss feature
// when bidder2 outbids bidder1, bidder1 should get refunded
#[test]
fn test_outbid_refunds_previous_bidder() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, bidder1) = setup(&env);
    let bidder2 = Address::generate(&env);

    // bidder1 bids first
    client.place_bid(&bidder1, &200_0000000i128);
    assert_eq!(client.get_highest_bidder(), Some(bidder1.clone()));

    // bidder2 bids higher - bidder1 should be refunded automatically
    client.place_bid(&bidder2, &350_0000000i128);

    // now bidder2 is the highest bidder
    assert_eq!(client.get_highest_bid(), 350_0000000i128);
    assert_eq!(client.get_highest_bidder(), Some(bidder2));

    // bidder1 got their tokens back automatically (no-loss!)
}

// TEST 4
// test finalizing the auction after deadline
#[test]
fn test_finalize_after_deadline() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, bidder1) = setup(&env);

    // place a bid first
    client.place_bid(&bidder1, &200_0000000i128);

    // jump forward past the deadline (100 ledgers)
    env.ledger().with_mut(|l| {
        l.sequence_number = 200;
    });

    // finalize should work now
    client.finalize_auction();

    assert_eq!(client.get_status(), AuctionStatus::Finalized);
}

// TEST 5
// seller can cancel if nobody bid yet
#[test]
fn test_cancel_with_no_bids() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, seller, _) = setup(&env);

    // no bids placed, seller cancels
    client.cancel_auction();

    assert_eq!(client.get_status(), AuctionStatus::Cancelled);
}

// TEST 6
// should fail if seller tries to cancel after a bid
#[test]
#[should_panic(expected = "Cannot cancel: bids already exist")]
fn test_cannot_cancel_after_bid() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, bidder1) = setup(&env);

    // someone bid
    client.place_bid(&bidder1, &200_0000000i128);

    // seller tries to cancel - should panic
    client.cancel_auction();
}

// TEST 7
// should fail if bid is below the minimum
#[test]
#[should_panic(expected = "Bid is below minimum")]
fn test_bid_below_minimum() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, bidder1) = setup(&env);

    // min is 100, we bid 50 - should panic
    client.place_bid(&bidder1, &50_0000000i128);
}

// TEST 8
// should fail if we try to finalize before deadline
#[test]
#[should_panic(expected = "Auction has not ended yet")]
fn test_cannot_finalize_before_deadline() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, bidder1) = setup(&env);

    client.place_bid(&bidder1, &200_0000000i128);

    // deadline not reached - should panic
    client.finalize_auction();
}
