#![allow(unused)]
use bitcoincore_rpc::bitcoin::{Address, Amount, BlockHash};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use std::fs::File;
use std::io::Write;

const RPC_URL: &str = "http://127.0.0.1:18443";
const RPC_USER: &str = "alice";
const RPC_PASS: &str = "password";

fn main() -> bitcoincore_rpc::Result<()> {
    let rpc = Client::new(RPC_URL, Auth::UserPass(RPC_USER.into(), RPC_PASS.into()))?;

    // Load or create 'Miner' wallet
    if !rpc.list_wallets()?.contains(&"Miner".to_string()) {
        rpc.create_wallet("Miner", None, None, None, None)?;
    }
    let miner = Client::new(
        &format!("{RPC_URL}/wallet/Miner"),
        Auth::UserPass(RPC_USER.into(), RPC_PASS.into()),
    )?;

    // Load or create 'Trader' wallet
    if !rpc.list_wallets()?.contains(&"Trader".to_string()) {
        rpc.create_wallet("Trader", None, None, None, None)?;
    }
    let trader = Client::new(
        &format!("{RPC_URL}/wallet/Trader"),
        Auth::UserPass(RPC_USER.into(), RPC_PASS.into()),
    )?;

    // Generate a new address for mining rewards
    let miner_addr = miner
        .get_new_address(Some("Mining Reward"), None)?
        .assume_checked();

    // Mine blocks until rewards are spendable (>= 100 confirmations required)
    let mut blocks = 0;
    while miner.get_balance(None, None)? <= Amount::from_btc(0.0).unwrap() {
        rpc.generate_to_address(1, &miner_addr)?;
        blocks += 1;
    }
    println!("Mined {blocks} blocks to get spendable balance");

    // Create a receiving address for the Trader
    let trader_addr = trader
        .get_new_address(Some("Received"), None)?
        .assume_checked();

    // Send 20 BTC from Miner to Trader
    let txid = miner.send_to_address(
        &trader_addr,
        Amount::from_btc(20.0).unwrap(),
        None,
        None,
        None,
        None,
        None,
        None,
    )?;

    // Check transaction is in mempool
    let _mempool_entry = rpc.get_mempool_entry(&txid)?;

    // Mine 1 block to confirm the transaction
    rpc.generate_to_address(1, &miner_addr)?;

    // Get transaction details
    let tx = miner.get_transaction(&txid, Some(true))?;
    let decoded = miner.get_raw_transaction_info(&txid, None)?;
    let blockhash: BlockHash = tx.info.blockhash.unwrap();
    let blockinfo = rpc.get_block_header_info(&blockhash)?;

    // Parse inputs and outputs
    let input_txid = &decoded.vin[0].txid;
    let input_vout = decoded.vin[0].vout;
    let raw_input_tx = miner.get_raw_transaction_info(input_txid.as_ref().unwrap(), None)?;
    let input_prevout = &raw_input_tx.vout[input_vout.unwrap() as usize];
    let miner_input_address = input_prevout.script_pub_key.address.as_ref().unwrap();
    let miner_input_amount = input_prevout.value;

    let mut trader_output_address = String::new();
    let mut trader_output_amount = 0.0;
    let mut miner_change_address = String::new();
    let mut miner_change_amount = 0.0;

    for out in &decoded.vout {
        let addr = out.script_pub_key.address.as_ref().unwrap();
        if addr == &trader_addr {
            trader_output_address = addr.clone().assume_checked().to_string();
            trader_output_amount = out.value.to_btc();
        } else {
            miner_change_address = addr.clone().assume_checked().to_string();
            miner_change_amount = out.value.to_btc();
        }
    }

    let fee = tx.fee.unwrap().to_btc();
    let block_height = blockinfo.height;

    // Write results to out.txt
    let mut file = File::create("out.txt")?;
    writeln!(file, "{txid}")?;
    let checked_address = miner_input_address.clone().assume_checked();
    writeln!(file, "{checked_address}")?;
    writeln!(file, "{miner_input_amount}")?;
    writeln!(file, "{trader_output_address}")?;
    writeln!(file, "{trader_output_amount}")?;
    writeln!(file, "{miner_change_address}")?;
    writeln!(file, "{miner_change_amount}")?;
    writeln!(file, "{fee}")?;
    writeln!(file, "{block_height}")?;
    writeln!(file, "{blockhash}")?;

    println!("out.txt generated successfully");

    Ok(())
}
