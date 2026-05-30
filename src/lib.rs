#![no_std]

// I am importing the tools I need from the Stellar Soroban SDK
use soroban_sdk::{
    contract, contractimpl, contracttype,
    token::Client as TokenClient,
    Address, Env, String, symbol_short,
};

// This tells us what state the auction is in
// I learned that enums are great for tracking status
#[contracttype]
#[derive(Clone, PartialEq)]
pub enum AuctionStatus {
    Active,     // auction is open and accepting bids
    Finalized,  // auction ended and winner was picked
    Cancelled,  // seller cancelled before any bids
}

// These are the keys I use to store data on the blockchain
// Think of them like variable names but stored on chain
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Seller,        // the person who created the auction
    TokenAddress,  // the SEP-41 token used for bidding
    ItemName,      // the name of the item being sold
    MinBid,        // the lowest bid allowed
    Deadline,      // when the auction stops (ledger number)
    HighestBidder, // who currently has the highest bid
    HighestBid,    // how much the highest bid is
    Status,        // is it Active, Finalized or Cancelled
}

// helper function to read the balance from storage
fn get_highest_bid_amount(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::HighestBid)
        .unwrap_or(0)
}

// my main contract struct
#[contract]
pub struct AuctionContract;

#[contractimpl]
impl AuctionContract {

    // CREATE AUCTION
    // the seller calls this to set up the auction
    // I set the deadline by adding duration to current ledger
    pub fn create_auction(
        env: Env,
        seller: Address,
        token: Address,
        item_name: String,
        min_bid: i128,
        duration_ledgers: u32,
    ) {
        // make sure seller signed this transaction
        seller.require_auth();

        // stop someone from calling this twice
        if env.storage().persistent().has(&DataKey::Seller) {
            panic!("Auction already created");
        }

        // min bid must be positive
        assert!(min_bid > 0, "Min bid must be more than zero");

        // calculate the deadline ledger
        let deadline = env.ledger().sequence() + duration_ledgers;

        // save everything to the blockchain
        env.storage().persistent().set(&DataKey::Seller, &seller);
        env.storage().persistent().set(&DataKey::TokenAddress, &token);
        env.storage().persistent().set(&DataKey::ItemName, &item_name);
        env.storage().persistent().set(&DataKey::MinBid, &min_bid);
        env.storage().persistent().set(&DataKey::Deadline, &deadline);
        env.storage().persistent().set(&DataKey::HighestBid, &0i128);
        env.storage().persistent().set(&DataKey::Status, &AuctionStatus::Active);

        // emit an event so the frontend can see it happened
        env.events().publish(
            (symbol_short!("created"),),
            (seller, item_name, min_bid, deadline),
        );
    }

    // PLACE BID
    // this is the no-loss part - if someone outbids you
    // your tokens come back to you automatically
    pub fn place_bid(env: Env, bidder: Address, amount: i128) {
        // bidder must sign
        bidder.require_auth();

        // read the current auction info
        let status: AuctionStatus = env
            .storage()
            .persistent()
            .get(&DataKey::Status)
            .expect("Auction not found");

        let deadline: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::Deadline)
            .expect("No deadline found");

        let min_bid: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::MinBid)
            .expect("No min bid found");

        let token_address: Address = env
            .storage()
            .persistent()
            .get(&DataKey::TokenAddress)
            .expect("No token found");

        let current_highest = get_highest_bid_amount(&env);

        // check auction is still running
        assert!(status == AuctionStatus::Active, "Auction is not active");

        // check deadline has not passed yet
        assert!(
            env.ledger().sequence() <= deadline,
            "Auction has ended"
        );

        // check bid meets the minimum
        assert!(amount >= min_bid, "Bid is below minimum");

        // check new bid is higher than current highest
        assert!(
            amount > current_highest,
            "Bid must be higher than current highest bid"
        );

        // get the token so we can move it around
        let token = TokenClient::new(&env, &token_address);

        // THIS IS THE NO-LOSS PART
        // if there was a previous bidder send their tokens back
        if current_highest > 0 {
            let prev_bidder: Address = env
                .storage()
                .persistent()
                .get(&DataKey::HighestBidder)
                .expect("No previous bidder");

            // refund the old bidder automatically
            token.transfer(
                &env.current_contract_address(),
                &prev_bidder,
                &current_highest,
            );

            env.events().publish(
                (symbol_short!("refunded"),),
                (prev_bidder, current_highest),
            );
        }

        // move new bid tokens from bidder into contract for safekeeping
        token.transfer_from(
            &env.current_contract_address(),
            &bidder,
            &env.current_contract_address(),
            &amount,
        );

        // save new highest bidder
        env.storage()
            .persistent()
            .set(&DataKey::HighestBidder, &bidder);
        env.storage()
            .persistent()
            .set(&DataKey::HighestBid, &amount);

        env.events().publish(
            (symbol_short!("bid"),),
            (bidder, amount),
        );
    }

    // FINALIZE AUCTION
    // called after deadline to send tokens to seller
    // and announce the winner
    pub fn finalize_auction(env: Env) {
        let status: AuctionStatus = env
            .storage()
            .persistent()
            .get(&DataKey::Status)
            .expect("Auction not found");

        let deadline: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::Deadline)
            .expect("No deadline");

        let highest_bid = get_highest_bid_amount(&env);

        // must still be active
        assert!(status == AuctionStatus::Active, "Auction is not active");

        // deadline must have passed
        assert!(
            env.ledger().sequence() > deadline,
            "Auction has not ended yet"
        );

        // there must be at least one bid
        assert!(highest_bid > 0, "No bids were placed");

        let seller: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Seller)
            .expect("No seller");

        let winner: Address = env
            .storage()
            .persistent()
            .get(&DataKey::HighestBidder)
            .expect("No winner found");

        let token_address: Address = env
            .storage()
            .persistent()
            .get(&DataKey::TokenAddress)
            .expect("No token");

        // send the winning bid to the seller
        let token = TokenClient::new(&env, &token_address);
        token.transfer(
            &env.current_contract_address(),
            &seller,
            &highest_bid,
        );

        // mark auction as done
        env.storage()
            .persistent()
            .set(&DataKey::Status, &AuctionStatus::Finalized);

        env.events().publish(
            (symbol_short!("finalized"),),
            (winner, seller, highest_bid),
        );
    }

    // CANCEL AUCTION
    // only the seller can cancel
    // and only if nobody has bid yet
    pub fn cancel_auction(env: Env) {
        let seller: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Seller)
            .expect("Auction not found");

        // only seller can do this
        seller.require_auth();

        let status: AuctionStatus = env
            .storage()
            .persistent()
            .get(&DataKey::Status)
            .expect("No status");

        let highest_bid = get_highest_bid_amount(&env);

        // must be active
        assert!(status == AuctionStatus::Active, "Auction is not active");

        // cannot cancel if bids exist
        assert!(
            highest_bid == 0,
            "Cannot cancel: bids already exist"
        );

        // mark as cancelled
        env.storage()
            .persistent()
            .set(&DataKey::Status, &AuctionStatus::Cancelled);

        env.events().publish(
            (symbol_short!("canceled"),),
            (seller,),
        );
    }

    // VIEW FUNCTIONS
    // these just read data, they dont cost gas

    pub fn get_highest_bid(env: Env) -> i128 {
        get_highest_bid_amount(&env)
    }

    pub fn get_highest_bidder(env: Env) -> Option<Address> {
        env.storage().persistent().get(&DataKey::HighestBidder)
    }

    pub fn get_deadline(env: Env) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::Deadline)
            .unwrap_or(0)
    }

    pub fn get_status(env: Env) -> AuctionStatus {
        env.storage()
            .persistent()
            .get(&DataKey::Status)
            .unwrap_or(AuctionStatus::Cancelled)
    }

    pub fn get_item_name(env: Env) -> String {
        env.storage()
            .persistent()
            .get(&DataKey::ItemName)
            .expect("Not initialized")
    }

    pub fn get_min_bid(env: Env) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::MinBid)
            .unwrap_or(0)
    }
}

// link the test file
mod test;
