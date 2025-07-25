use bitcoin::Amount;
use bitcoincore_rpc::{Auth, Client, RpcApi, json};
use std::fs::File;
use std::io::Write;

const RPC_URL: &str = "http://127.0.0.1:18443";
const RPC_USER: &str = "alice";
const RPC_PASS: &str = "password";

fn main() -> bitcoincore_rpc::Result<()> {
    let rpc = Client::new(RPC_URL, Auth::UserPass(RPC_USER.to_string(), RPC_PASS.to_string()))?;

    // Create or load 'Miner' wallet
    if rpc.list_wallets()?.iter().all(|w| w != "Miner") {
        rpc.create_wallet("Miner", None, None, None, None)?;
    }
    let miner_rpc = Client::new(&format!("{}/wallet/Miner", RPC_URL), Auth::UserPass(RPC_USER.to_string(), RPC_PASS.to_string()))?;

    // Create or load 'Trader' wallet
    if rpc.list_wallets()?.iter().all(|w| w != "Trader") {
        rpc.create_wallet("Trader", None, None, None, None)?;
    }
    let trader_rpc = Client::new(&format!("{}/wallet/Trader", RPC_URL), Auth::UserPass(RPC_USER.to_string(), RPC_PASS.to_string()))?;

    // Miner: get address and mine blocks to it
    let miner_address = miner_rpc.get_new_address(Some("Mining Reward"), None)?;
    let mut blocks_mined = 0;
    while miner_rpc.get_balance(None, None)? <= Amount::from_btc(0.0)? {
        rpc.generate_to_address(1, &miner_address)?;
        blocks_mined += 1;
    }

    // 💡 Why wallet balance behaves this way:
    // Coinbase rewards need 100 confirmations to mature in regtest.
    // Hence, we need to mine 101 blocks to get a spendable balance.

    println!("Blocks mined to get spendable balance: {}", blocks_mined);
    println!("Miner Balance: {:?}", miner_rpc.get_balance(None, None)?);

    // Trader: generate receiving address
    let trader_address = trader_rpc.get_new_address(Some("Received"), None)?;

    // Send 20 BTC from Miner to Trader
    let txid = miner_rpc.send_to_address(
        &trader_address,
        Amount::from_btc(20.0)?,
        None, None, None, None, None, None,
    )?;

    // Get unconfirmed tx from mempool
    let mempool_entry = rpc.get_mempool_entry(&txid)?;
    println!("Mempool entry: {:?}", mempool_entry);

    // Confirm transaction by mining 1 block
    rpc.generate_to_address(1, &miner_address)?;

    // Fetch transaction details
    let tx_detail = miner_rpc.get_transaction(&txid, Some(true))?;
    let decoded_tx = tx_detail.details;
    let block_hash = tx_detail.info.blockhash.unwrap();
    let block = rpc.get_block_header_info(&block_hash)?;
    let block_height = block.height;

    let raw_tx = miner_rpc.get_raw_transaction_info(&txid, Some(true))?;

    let mut input_address = String::new();
    let mut input_amount = 0.0;
    let mut output_trader_address = String::new();
    let mut output_trader_amount = 0.0;
    let mut change_address = String::new();
    let mut change_amount = 0.0;

    for vin in &raw_tx.vin {
        if let Some(prevout) = &vin.prevout {
            input_address = prevout.script_pub_key.address.clone().unwrap_or_default();
            input_amount = prevout.value.to_btc();
        }
    }

    for vout in &raw_tx.vout {
        let value = vout.value.to_btc();
        if let Some(addresses) = &vout.script_pub_key.addresses {
            let addr = &addresses[0];
            if addr == &trader_address.to_string() {
                output_trader_address = addr.clone();
                output_trader_amount = value;
            } else {
                change_address = addr.clone();
                change_amount = value;
            }
        }
    }

    let fee = tx_detail.fee.to_btc();

    // Write output file
    let mut file = File::create("out.txt")?;
    writeln!(file, "{}", txid)?;
    writeln!(file, "{}", input_address)?;
    writeln!(file, "{}", input_amount)?;
    writeln!(file, "{}", output_trader_address)?;
    writeln!(file, "{}", output_trader_amount)?;
    writeln!(file, "{}", change_address)?;
    writeln!(file, "{}", change_amount)?;
    writeln!(file, "{}", fee)?;
    writeln!(file, "{}", block_height)?;
    writeln!(file, "{}", block_hash)?;

    println!("✅ Transaction details written to out.txt");

    Ok(())
}

