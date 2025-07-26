use bitcoincore_rpc::bitcoin::{Amount, BlockHash, SignedAmount};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use std::fs::File;
use std::io::Write;

const RPC_URL: &str = "http://127.0.0.1:18443";
const RPC_USER: &str = "alice";
const RPC_PASS: &str = "password";

fn main() -> bitcoincore_rpc::Result<()> {
    let rpc = Client::new(RPC_URL, Auth::UserPass(RPC_USER.into(), RPC_PASS.into()))?;

    let miner_wallet_name = "Miner";
    let trader_wallet_name = "Trader";

    let loaded_wallets = rpc.list_wallets()?;
    if !loaded_wallets.contains(&miner_wallet_name.to_string())
        && rpc.load_wallet(miner_wallet_name).is_err()
    {
        rpc.create_wallet(miner_wallet_name, None, None, None, None)?;
    }

    let miner = Client::new(
        &format!("{RPC_URL}/wallet/{miner_wallet_name}"),
        Auth::UserPass(RPC_USER.into(), RPC_PASS.into()),
    )?;

    if !rpc
        .list_wallets()?
        .contains(&trader_wallet_name.to_string())
        && rpc.load_wallet(trader_wallet_name).is_err()
    {
        rpc.create_wallet(trader_wallet_name, None, None, None, None)?;
    }

    let trader = Client::new(
        &format!("{RPC_URL}/wallet/{trader_wallet_name}"),
        Auth::UserPass(RPC_USER.into(), RPC_PASS.into()),
    )?;

    let miner_reward_addr = miner
        .get_new_address(Some("Mining Reward"), None)?
        .assume_checked();

    let mut mined_blocks = 0;
    while miner.get_balance(None, None)? <= Amount::from_btc(0.0).unwrap() {
        rpc.generate_to_address(1, &miner_reward_addr)?;
        mined_blocks += 1;
    }
    println!("Mined {mined_blocks} blocks to get spendable balance");

    let trader_addr = trader
        .get_new_address(Some("Received"), None)?
        .assume_checked();

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

    // Check that tx enters mempool
    let _ = rpc.get_mempool_entry(&txid)?;

    // Mine 6 blocks to confirm the transaction
    rpc.generate_to_address(6, &miner_reward_addr)?;

    // Fetch transaction info
    let tx = miner.get_transaction(&txid, Some(true))?;
    let decoded = miner.get_raw_transaction_info(&txid, None)?;
    let block_hash: BlockHash = tx.info.blockhash.unwrap();
    let block_info = rpc.get_block_header_info(&block_hash)?;

    let input_txid = decoded.vin[0].txid.as_ref().expect("Expected txid in vin");
    let input_vout = decoded.vin[0].vout.expect("Expected vout in vin");

    let raw_input_tx = miner.get_raw_transaction_info(input_txid, None)?;
    let prevout = &raw_input_tx.vout[input_vout as usize];

    let miner_input_address = prevout
        .script_pub_key
        .address
        .as_ref()
        .expect("Input address missing")
        .clone()
        .assume_checked();
    let miner_input_amount = prevout.value.to_btc();

    let mut trader_output_address = String::new();
    let mut trader_output_amount = 0.0;
    let mut miner_change_address = String::new();
    let mut miner_change_amount = 0.0;

    for out in &decoded.vout {
        let addr = out
            .script_pub_key
            .address
            .as_ref()
            .expect("Output address missing")
            .clone()
            .assume_checked()
            .to_string();

        if addr == trader_addr.to_string() {
            trader_output_address = addr;
            trader_output_amount = out.value.to_btc();
        } else {
            miner_change_address = addr;
            miner_change_amount = out.value.to_btc();
        }
    }

    let fee = tx
        .details
        .iter()
        .map(|d| d.fee.unwrap_or_default())
        .sum::<SignedAmount>()
        .to_btc();

    let block_height = block_info.height;

    let mut file = File::create("../../out.txt")?;
    writeln!(file, "{txid}")?;
    writeln!(file, "{miner_input_address}")?;
    writeln!(file, "{miner_input_amount}")?;
    writeln!(file, "{trader_output_address}")?;
    writeln!(file, "{trader_output_amount}")?;
    writeln!(file, "{miner_change_address}")?;
    writeln!(file, "{miner_change_amount}")?;
    writeln!(file, "{fee:.8}")?;
    writeln!(file, "{block_height}")?;
    writeln!(file, "{block_hash}")?;

    println!("out.txt generated successfully");
    Ok(())
}
