# Ogre - The Ore Miners Collective Miner

WARNING: THIS DOES NOT HAVE WITHDRAW/CLAIM IMPLEMENTED YET.

## Setup
This miner is fully configured in src/config.rs

1. Set up the keypair you will mine from. (Funding and Funding_pk)
2. Set up the custom RPC you will use (Jito doesn't have to be jito, it can be any sendTransaction rpc. can also be the same as your other RPC)
3. IF you use the jito sendTransaction endpoint, enable tips by setting the include_tip var to true  ( bundles not supported atm )
4. To be able to squeeze 20 ix in your tx, you need to set up ALT (todo: automatic?)
5. Adjust MINERCOUNT. These are the number of keypairs you will mine for. More is better.
6. Adjust MINERLIMIT. These are the number of keypairs you will put into one tx. In the beginning, set this to 5. Once all/most of your miners are registered, set this to 20




```
cargo build --release
target/build/ogre
```
