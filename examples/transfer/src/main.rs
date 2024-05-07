use aleo_agent::account::Account;
use aleo_agent::agent::{Agent, TransferArgs, TransferType};
use aleo_agent::{CiphertextRecord, PlaintextRecord, MICROCREDITS};
use anyhow::Result;
use std::str::FromStr;

fn main() -> Result<()> {
    // private key format: APrivateKey1zkp...
    let alice_key = "Alice PRIVATE KEY";
    let alice_account = Account::from_private_key(alice_key)?;
    let alice_agent = Agent::builder().with_account(alice_account).build();

    let bob_key = "Bob PRIVATE KEY";
    let bob_account = Account::from_private_key(bob_key)?;
    let bob_address = bob_account.address();
    
    // get alice public balance
    let public_balance = alice_agent.get_public_balance()?;
    println!("Alice Public Balance : {}", public_balance);
    
    // public transfer to public account
    let transfer_args = TransferArgs::from(
        MICROCREDITS, // 1 credit
        bob_address.to_owned(),
        1,
        None,
        TransferType::Public,
    );
    let tx_hash = alice_agent.transfer(transfer_args)?;
    println!("tx_hash: {tx_hash}");

    // public transfer to private
    let transfer_args = TransferArgs::from(
        MICROCREDITS, // 1 credit
        bob_address.to_owned(),
        1,
        None,
        TransferType::PublicToPrivate,
    );
    let tx_hash = alice_agent.transfer(transfer_args)?;
    println!("tx_hash: {tx_hash}");

    // private transfer to public
    // plaintext record format : "{owner: aleo1xxxxx.private,microcredits: 1u64.private,_nonce: xxxxxgroup.public}"
    let record = PlaintextRecord::from_str("PLAINTEXT RECORD")?;

    //  decrypt plaintext record from ciphertext record
    //  format: record1xxxxxxxxxx
    let ciphertext_record = CiphertextRecord::from_str("CIPHERTEXT RECORD")?;
    let fee_record = alice_agent.decrypt_ciphertext_record(&ciphertext_record)?;

    let transfer_args = TransferArgs::from(
        MICROCREDITS, // 1 credit
        bob_address.to_owned(),
        10,
        Some(fee_record),
        TransferType::PrivateToPublic(record),
    );
    let tx_hash = alice_agent.transfer(transfer_args)?;
    println!("tx_hash: {tx_hash}");

    // private transfer to private
    let encrypted_fee_record = CiphertextRecord::from_str("record1xxxxxxx")?;
    let record = alice_agent.decrypt_ciphertext_record(&encrypted_fee_record)?;

    let transfer_args = TransferArgs::from(
        MICROCREDITS,
        bob_address.to_owned(),
        1,
        None,
        TransferType::Private(record),
    );
    let tx_hash = alice_agent.transfer(transfer_args)?;
    println!("tx_hash: {tx_hash}");

    Ok(())
}
