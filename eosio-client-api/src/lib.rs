// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]
//

extern crate reqwest;
extern crate serde_json;
//#[macro_use]
extern crate error_chain;
#[macro_use] extern crate lazy_static;


extern crate base64;
extern crate bs58;
extern crate ripemd160;
extern crate eosio_client_keys;
mod numeric;
pub mod api_types;
pub mod json_rpc;
pub mod wasm;
mod abi;
//mod serialize;
pub mod wallet_types;
pub mod errors;

#[cfg(test)]
mod tests {
    // TBD
}
