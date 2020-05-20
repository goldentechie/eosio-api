use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde_json::Value;
use reqwest::header::{CONTENT_TYPE, HeaderValue};
use crate::errors::{Result, ErrorKind};
use crate::api_types::{GetAccount, GetAbi, RequiredKeys, GetInfo, TransactionIn, ActionIn, AuthorizationIn, ErrorReply, PackedTransactionIn, GetCodeHash, GetRawABI};
use crate::wallet_types::Wallet;
use eosio_keys::{EOSPublicKey, EOSPrivateKey, EOSSignature};
use crate::wasm::WASM;
use crate::abi::ABIName;
//use crate::numeric::binary_to_base58;

use chrono::{Utc, DateTime};
use std::time::Instant;


pub struct EOSRPC {
    pub client: Client,
    pub host: String,
}

impl EOSRPC {
    pub fn blocking(host: String) -> EOSRPC {
        let client = reqwest::blocking::Client::new();
        EOSRPC { client, host }
    }

    pub fn blocking_req(&self, url: &str, in_json: Value) -> Result<String> {
        let full_url = [&self.host, url].concat();
        let req = self.client.post(&full_url).json(&in_json);
        let response = req.send()?;
        let content_type = response.headers().get(CONTENT_TYPE).unwrap();
        let hv_json = HeaderValue::from_static("application/json");
        if content_type != hv_json {
            return Err(ErrorKind::InvalidResponseContentType.into());
        }
        let status = response.status();

        if status == StatusCode::OK || status == StatusCode::CREATED || status == StatusCode::ACCEPTED {
            Ok(response.text()?)
        } else {
            // TODO return the JSON error
            let tx: &str = &response.text()?;
            let msg: ErrorReply = serde_json::from_str(tx).unwrap();
            //    eprintln!("{:#?}", msg);
            Err(ErrorKind::InvalidResponseStatus(msg.error).into())
        }
    }

    pub fn get_account(&self, account_name: &str) -> Result<GetAccount> {
        let value = serde_json::json!({ "account_name": account_name });
        let res = self.blocking_req("/v1/chain/get_account", value)?;
        let ga: GetAccount = serde_json::from_str(&res).unwrap();
        Ok(ga)
    }
    pub fn get_abi(&self, account_name: &str) -> Result<GetAbi> {
        let value = serde_json::json!({ "account_name": account_name });
        let res = self.blocking_req("/v1/chain/get_abi", value)?;
        let ga: GetAbi = serde_json::from_str(&res).unwrap();
        Ok(ga)
    }

    pub fn get_info(&self) -> Result<GetInfo> {
        let value = serde_json::json!({ "this is": "the future" });

        let res = self.blocking_req("/v1/chain/get_info", value)?;
        let ga: GetInfo = serde_json::from_str(&res).unwrap();
        Ok(ga)
    }

    pub fn get_code_hash(&self, account_name: &str) -> Result<GetCodeHash> {
        let value = serde_json::json!({ "account_name": account_name });

        let res = self.blocking_req("/v1/chain/get_code_hash", value)?;
        let gc: GetCodeHash = serde_json::from_str(&res)?;
        Ok(gc)
    }
    pub fn get_raw_abi(&self, account_name: &str) -> Result<GetRawABI> {
        let value = serde_json::json!({ "account_name": account_name });

        let res = self.blocking_req("/v1/chain/get_raw_abi", value)?;
        let gr: GetRawABI = serde_json::from_str(&res)?;
        Ok(gr)
    }

    pub fn get_required_keys(&self, transaction: &TransactionIn, keys: Vec<EOSPublicKey>) -> Result<RequiredKeys> {
        let mut key_str: Vec<String> = vec![];
        for key in keys {
            let x = key.to_eos_string()?;
            key_str.push(x);
        }

        let value = serde_json::json!({ "transaction": transaction, "available_keys":key_str});
        let res = self.blocking_req("/v1/chain/get_required_keys", value)?;
        let rk: RequiredKeys = serde_json::from_str(&res).unwrap();
        Ok(rk)
    }


    pub fn push_transaction(&self, wallet: Wallet, action: ActionIn, ref_block_num: usize, ref_block_prefix: usize, exp_time: DateTime<Utc>) -> Result<String> {
        let ti = TransactionIn::simple(action, ref_block_num, ref_block_prefix, exp_time);
        let packed_trx = serde_json::to_string(&ti)?;
        let trx = vec_u8_to_str(&packed_trx.as_bytes().to_vec())?;

        let pubkeys = wallet.keys()?;
        let required_keys = self.get_required_keys(&ti, pubkeys)?;
        let eospubs: Vec<EOSPublicKey> = EOSPublicKey::from_eos_strings(&required_keys.required_keys)?;

        let signed_transaction = wallet.sign_transaction(ti, eospubs)?;
        let in_val = serde_json::json!(PackedTransactionIn{
            signatures: signed_transaction.signatures,
            compression: "none".to_string(),
            packed_context_free_data: "".to_string(),
            packed_trx: trx,
        });
        match self.blocking_req("/v1/chain/push_transaction", in_val) {
            Err(e) => {
                eprintln!("{:#?}", e);
                panic!("push tran");
                //Err("FAIL".into())
            }
            Ok(s) => {
                eprintln!("{}", s);
                Ok(s)
            }
        }
    }

    #[allow(dead_code)]
    fn push_transaction_int(&self, private_key: EOSPrivateKey, action: ActionIn, ref_block_num: usize, ref_block_prefix: usize, exp_time: DateTime<Utc>) -> Result<()> {
       eprintln!("push_transaction_int does not work. use push_transaction");
        let now = Instant::now();

        let ti = TransactionIn::simple(action, ref_block_num, ref_block_prefix, exp_time);
        let packed_trx = serde_json::to_string(&ti)?;

        let sig: EOSSignature = private_key.sign(packed_trx.as_bytes())?;
         // let valid_sig = "SIG_K1_KVLKbA96J7egJfZP56ddqFy6t2EAJR57bAd9vTnuYJS6S9exPA3GZkVCzvT5XrfWLVSYuBikYFiAKLQXWVguxYFovfmZJg";
        let sig_str = sig.to_eos_string().unwrap();
        // eprintln!("SIG {} {}", sig_str, sig_str.len());
        // eprintln!("EQ? {} {}", valid_sig, valid_sig.len());
        let trx = vec_u8_to_str(&packed_trx.as_bytes().to_vec())?;

        let in_val = serde_json::json!(PackedTransactionIn{
            signatures: vec![sig_str],
            compression: "none".to_string(),
            packed_context_free_data: "".to_string(),
            packed_trx: trx,
        });

        match self.blocking_req("/v1/chain/push_transaction", in_val) {
            Err(e) => {
                eprintln!("PT-6 {:?}", now.elapsed());
                eprintln!("{:#?}", e);
                panic!("Error Push Tran")
            }
            Ok(s) => {
                eprintln!("{}", s)
            }
        }

        Ok(())
    }
}

struct ActionSetcodeData {
    name: ABIName,
    vmtype: u8,
    vmversion: u8,
    wasm: WASM,
}

fn byte_to_char(x: u8) -> char {
    (if x <= 9 {
        x + b'0'
    } else {
        x - 10 + b'a'
    }) as char
}

pub fn vec_u8_to_str(out: &[u8]) -> Result<String> {
    let mut str = String::with_capacity(out.len());
    for x in out {
        str.push(byte_to_char((x & 0xf0).checked_shr(4).unwrap_or(0)));
        str.push(byte_to_char(x & 0x0f));
    }
    Ok(str)
}

impl ActionSetcodeData {
    fn to_str(&self) -> Result<String> {
       // let code_len = self.wasm.code.len();
        let buf = self.name.value.to_ne_bytes().to_vec();
// let out:Vec<u8> = Vec::<u8>::with_capacity(code_len+buf.len()+2);
        let vm: Vec<u8> = [self.vmtype, self.vmversion].to_vec();
        let c = self.wasm.code.to_vec();
        let out = [buf, vm, c].concat();
        Ok(vec_u8_to_str(&out)?)
    }
}

pub fn create_setcode_action(name: ABIName, code: WASM) -> Result<ActionIn> {
    let auth = AuthorizationIn { permission: "active".to_string(), actor: name.to_str()? };
    let v_auth: Vec<AuthorizationIn> = vec![auth];
    let data = ActionSetcodeData { name, vmtype: 0, vmversion: 0, wasm: code }.to_str()?;

    Ok(ActionIn {
        name: "setcode".to_string(),
        account: "eosio".to_string(),
        data,
        authorization: v_auth,
    })
}

#[cfg(test)]
mod test {
    use super::*;

    //use crate::api_types::GetAccount;
    use crate::wallet_types::{get_wallet_pass, EOSIO_CHAIN_ID};
    use chrono::{NaiveDateTime, Duration};

    //const TEST_HOST: &str = "http://127.0.0.1:8888";
    const TEST_HOST: &str = "https://api.testnet.eos.io";
    const TEST_KEOSD: &str = "http://127.0.0.1:3888";

    const TEST_WALLET_NAME: &str = "default";
//const TEST_HOST: &str = "https://eos.greymass.com";
//const TEST_HOST: &str = "https://chain.wax.io";

    #[test]
    fn blocking_req_test() -> Result<()> {
        let eos = EOSRPC::blocking(String::from(TEST_HOST));
        let _ga = eos.get_account("eosio")?;

        let _abi = eos.get_abi("eosio")?;
        Ok(())
    }

    #[test]
    fn blocking_get_info() -> Result<()>{
        let eos = EOSRPC::blocking(String::from(TEST_HOST));
        let _gi = eos.get_info()?;
        Ok(())
    }

    #[test]
    fn datetime_format() {
        let s = "2020-05-16T05:12:03";
        const FORMAT: &'static str = "%Y-%m-%dT%H:%M:%S";
       // let _tz_offset = FixedOffset::east(0);
        match NaiveDateTime::parse_from_str(s, FORMAT) {
            Err(_e) => {
                eprintln!("{:#?}", _e);
                assert!(false)
            }
            Ok(_dt) => {
                assert!(true)
            }
        }
    }

    #[test]
    fn blocking_get_required_keys() -> Result<()> {
      //  let client = reqwest::blocking::Client::new();
        let eos = EOSRPC::blocking(String::from(TEST_HOST));
        let keys = vec![
            EOSPublicKey::from_eos_string("EOS6zUgp7uAV1pCTXZMGJyH3dLUSWJUkZWGA9WpWxyP2pCT3mAkNX").unwrap(),
            EOSPublicKey::from_eos_string("EOS7ctUUZhtCGHnxUnh4Rg5eethj3qNS5S9fijyLMKgRsBLh8eMBB").unwrap(),
        ];
        let gi: GetInfo = eos.get_info()?;
        let exp_time = gi.head_block_time + Duration::days(1);
        match WASM::read_file("test/good.wasm") {
            Err(_) => assert!(false),
            Ok(wasm) => {
                let name = ABIName::from_str("fwonhjnefmps").unwrap();
                let action = create_setcode_action(name, wasm)?;
                let ti = TransactionIn::simple(action, gi.head_block_num, 0, exp_time);
                let rk = eos.get_required_keys(&ti, keys).unwrap();
                assert!(rk.required_keys.len() > 0);
                let k = &rk.required_keys[0];
                assert_eq!(k, "EOS7ctUUZhtCGHnxUnh4Rg5eethj3qNS5S9fijyLMKgRsBLh8eMBB");
                // eprintln!("{:#?}", rk);
            }
        }
        Ok(())
    }

    #[test]
    fn blocking_push_txn() -> Result<()> {
        let eos = EOSRPC::blocking(String::from(TEST_HOST));
        let wallet = Wallet::create_with_chain_id(EOSRPC::blocking(String::from(TEST_KEOSD)), EOSIO_CHAIN_ID);
        let wallet_pass = get_wallet_pass()?;
        wallet.unlock(&TEST_WALLET_NAME, &wallet_pass)?;

        let gi: GetInfo = eos.get_info()?;
        let exp_time = gi.head_block_time + Duration::days(1);

        let _key = EOSPrivateKey::from_string("PVT_K1_2jH3nnhxhR3zPUcsKaWWZC9ZmZAnKm3GAnFD1xynGJE1Znuvjd")?;
        let wasm = WASM::read_file("test/good.wasm")?;

        let name = ABIName::from_str("fwonhjnefmps").unwrap();
        let action = create_setcode_action(name, wasm)?;
        let _res = eos.push_transaction(wallet, action, gi.head_block_num, 0, exp_time)?;
        eprintln!("{:#?}", "hi");

        Ok(())
    }

    #[test]
    #[ignore]
    fn blocking_push_txn_internal() -> Result<()> {
        let eos = EOSRPC::blocking(String::from(TEST_HOST));
        let gi: GetInfo = eos.get_info()?;
        let exp_time = gi.head_block_time + Duration::days(1);

        let key = EOSPrivateKey::from_string("PVT_K1_2jH3nnhxhR3zPUcsKaWWZC9ZmZAnKm3GAnFD1xynGJE1Znuvjd")?;
        let wasm = WASM::read_file("test/good.wasm")?;

        let name = ABIName::from_str("fwonhjnefmps").unwrap();
        let action = create_setcode_action(name, wasm)?;
        let _res = eos.push_transaction_int(key, action, gi.head_block_num, 0, exp_time)?;

        Ok(())
    }
    #[test]
    fn blocking_get_raw_abi() -> Result<()> {
        let eos = EOSRPC::blocking(String::from(TEST_HOST));
        let _res = eos.get_raw_abi("eosio")?;

        Ok(())
    }
    #[test]
    fn blocking_packed() -> Result<()> {
        let _packed_action = "000000008090b1ca000000000091b1ca000075982aea3055";
        let _raw_action = "'{\"account\":\"test1\", \"code\":\"test2\", \"type\":\"eosioeosio\"}'";
        let _raw_txn = "{
  \"expiration\": \"2018-08-02T20:24:36\",
  \"ref_block_num\": 14207,
  \"ref_block_prefix\": 1438248607,
  \"max_net_usage_words\": 0,
  \"max_cpu_usage_ms\": 0,
  \"delay_sec\": 0,
  \"context_free_actions\": [],
  \"actions\": [{
      \"account\": \"eosio\",
      \"name\": \"newaccount\",
      \"authorization\": [{
          \"actor\": \"eosio\",
          \"permission\": \"active\"
        }
      ],
      \"data\": \"0000000000ea305500a6823403ea30550100000001000240cc0bf90a5656c8bb81f0eb86f49f89613c5cd988c018715d4646c6bd0ad3d8010000000100000001000240cc0bf90a5656c8bb81f0eb86f49f89613c5cd988c018715d4646c6bd0ad3d801000000\"
    }
  ],
  \"transaction_extensions\": []
}";
        let _packed_trx = "8468635b7f379feeb95500000000010000000000ea305500409e9a2264b89a010000000000ea305500000000a8ed3232660000000000ea305500a6823403ea30550100000001000240cc0bf90a5656c8bb81f0eb86f49f89613c5cd988c018715d4646c6bd0ad3d8010000000100000001000240cc0bf90a5656c8bb81f0eb86f49f89613c5cd988c018715d4646c6bd0ad3d80100000000";
        let _packed_trx_json = "
        {
            \"signatures\": [],
            \"compression\": \"none\",
            \"packed_context_free_data\": \"\",
            \"packed_trx\": \"8468635b7f379feeb95500000000010000000000ea305500409e9a2264b89a010000000000ea305500000000a8ed3232660000000000ea305500a6823403ea30550100000001000240cc0bf90a5656c8bb81f0eb86f49f89613c5cd988c018715d4646c6bd0ad3d8010000000100000001000240cc0bf90a5656c8bb81f0eb86f49f89613c5cd988c018715d4646c6bd0ad3d80100000000\"
        }";

        Ok(())
    }
}

