# no-loss-auction
 Week 3 - No Loss Auction Protocol - Stellar Kenya Bootcamp
# No-Loss Auction Protocol 🏆
### Stellar Kenya Blockchain Bootcamp — Week 3

---

## What I Built
A decentralized auction on the Stellar blockchain where nobody loses
their tokens. When someone outbids you, your tokens are sent back to
your wallet automatically. The winner pays, everyone else gets refunded.

## How It Works
1. Seller creates an auction with an item name and minimum bid
2. Bidders place bids using SEP-41 tokens
3. If someone outbids you, your tokens come back automatically
4. After the deadline, anyone can finalize the auction
5. The seller receives the winning bid tokens
6. The winner gets the item

## Functions in the Smart Contract

| Function | Who calls it | What it does |
|---|---|---|
| create_auction | Seller | Sets up the auction |
| place_bid | Anyone | Places a bid, refunds previous bidder |
| finalize_auction | Anyone | Closes auction after deadline |
| cancel_auction | Seller only | Cancels if no bids placed yet |
| get_highest_bid | Anyone | Read current highest bid |
| get_status | Anyone | Read auction status |

## Test Cases — 8 Total
- Test 1: Create auction saves correct data
- Test 2: Place bid works and saves highest bidder
- Test 3: Getting outbid triggers automatic refund
- Test 4: Finalize works after deadline
- Test 5: Seller can cancel if no bids
- Test 6: Cannot cancel if bids exist
- Test 7: Cannot bid below minimum
- Test 8: Cannot finalize before deadline

## Deployed Contract
- Network: Stellar Testnet
- Contract ID: `PASTE YOUR CONTRACT ID HERE`

## Challenges I Faced
The trickiest part was understanding how the no-loss refund works.
When a new bid comes in I had to send the old bidder's tokens back
before saving the new bidder. I also had to learn how ledger numbers
work as a deadline instead of timestamps like in Solidity.

## Files
- Cargo.toml — dependencies
- src/lib.rs  — smart contract
- src/test.rs — 8 test cases  
- frontend/index.html — frontend UI

## How to Run
cargo test
stellar contract build
