use bitcoincore_rpc::bitcoin::{Amount, BlockHash, SignedAmount};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use std::fs::File;
use std::io::Write;

const RPC_URL: &str = "http://127.0.0.1:18443";
const RPC_USER: &str = "alice";
const RPC_PASS: &str = "password";

fn main() -> bitcoincore_rpc::Result<()> {
    let rpc = Client::new(RPC_URL, Auth::UserPass(RPC_USER.into(), RPC_PASS.into()))?;

    let miner_wallet = "Miner";
    let trader_wallet = "Trader";

    // Load or create miner wallet
    if !rpc.list_wallets()?.contains(&miner_wallet.to_string())
        && rpc.load_wallet(miner_wallet).is_err()
    {
        rpc.create_wallet(miner_wallet, None, None, None, None)?;
    }

    let miner = Client::new(
        &format!("{RPC_URL}/wallet/{miner_wallet}"),
        Auth::UserPass(RPC_USER.into(), RPC_PASS.into()),
    )?;

    // Load or create trader wallet
    if !rpc.list_wallets()?.contains(&trader_wallet.to_string())
        && rpc.load_wallet(trader_wallet).is_err()
    {
        rpc.create_wallet(trader_wallet, None, None, None, None)?;
    }

    let trader = Client::new(
        &format!("{RPC_URL}/wallet/{trader_wallet}"),
        Auth::UserPass(RPC_USER.into(), RPC_PASS.into()),
    )?;

    // Generate initial miner address
    let miner_reward_addr = miner
        .get_new_address(Some("mining_reward"), None)?
        .assume_checked();

    // Mine until we have funds
    let mut mined_blocks = 0;
    while miner.get_balance(None, None)? <= Amount::from_btc(0.0).unwrap() {
        rpc.generate_to_address(1, &miner_reward_addr)?;
        mined_blocks += 1;
    }

    println!("Mined {mined_blocks} blocks to get spendable balance");

    // Generate trader receive address
    let trader_receive_addr = trader
        .get_new_address(Some("trader_receive"), None)?
        .assume_checked();

    // Miner sends 20 BTC to trader
    let txid = miner.send_to_address(
        &trader_receive_addr,
        Amount::from_btc(20.0).unwrap(),
        None,
        None,
        None,
        None,
        None,
        None,
    )?;

    // Ensure tx entered mempool
    let _ = rpc.get_mempool_entry(&txid)?;

    // Confirm the transaction
    rpc.generate_to_address(6, &miner_reward_addr)?;

    // Get decoded transaction info
    let tx = miner.get_transaction(&txid, Some(true))?;
    let decoded = miner.get_raw_transaction_info(&txid, None)?;
    let block_hash: BlockHash = tx.info.blockhash.unwrap();
    let block_info = rpc.get_block_header_info(&block_hash)?;

    // Get input (vin) info
    let input_txid = decoded.vin[0].txid.as_ref().unwrap();
    let input_vout = decoded.vin[0].vout.unwrap();
    let input_tx = miner.get_raw_transaction_info(input_txid, None)?;
    let input_prevout = &input_tx.vout[input_vout as usize];

    let miner_input_address = input_prevout
        .script_pub_key
        .address
        .as_ref()
        .unwrap()
        .clone()
        .assume_checked();

    let miner_input_amount = input_prevout.value.to_btc();

    // Prepare to extract outputs
    let mut trader_output_address = String::new();
    let mut trader_output_amount = 0.0;
    let mut miner_change_address = String::new();
    let mut miner_change_amount = 0.0;

    for vout in &decoded.vout {
        let address = vout
            .script_pub_key
            .address
            .as_ref()
            .unwrap()
            .clone()
            .assume_checked()
            .to_string();

        if address == trader_receive_addr.to_string() {
            trader_output_address = address;
            trader_output_amount = vout.value.to_btc();
        } else {
            miner_change_address = address;
            miner_change_amount = vout.value.to_btc();
        }
    }

    // Sum of all fees (in SignedAmount)
    let fee: f64 = tx
        .details
        .iter()
        .map(|d| d.fee.unwrap_or_default())
        .sum::<SignedAmount>()
        .to_btc();

    let block_height = block_info.height;

    // Write to output file
    let mut file = File::create("../../out.txt")?;
    writeln!(file, "{txid}")?;
    writeln!(file, "{miner_input_address}")?;
    writeln!(file, "{miner_input_amount}")?;
    writeln!(file, "{trader_output_address}")?;
    writeln!(file, "{trader_output_amount}")?;
    writeln!(file, "{miner_change_address}")?;
    writeln!(file, "{miner_change_amount}")?;
    writeln!(file, "{fee:e}")?;
    writeln!(file, "{block_height}")?;
    writeln!(file, "{block_hash}")?;

    println!("out.txt generated successfully");
    Ok(())
}
