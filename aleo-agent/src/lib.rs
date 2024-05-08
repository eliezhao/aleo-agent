//! The `aleo-agent` is a simple-to-use library that enables you to
//! build applications and interact with the [Aleo Network](https://aleo.org) using Rust.
//!
//! ## Overview
//! The `aleo-agent` provides a set of tools for deploying and executing programs, as well as
//! tools for communicating with the Aleo Network.
//!
//! The agent is designed to expose both low-level APIs for communicating with the
//! Aleo Network [Node API](https://github.com/AleoHQ/snarkOS/blob/fc340c679960e63612c536d69e71405b77e113f4/node/rest/src/lib.rs#L131) 
//! and higher-level APIs for communicating with deployed programs.
//!
//! ## Example
//!
//! In this example, a call to the Aleo network demonstrates how to create an agent to access its own public balance and
//! transfer 1 credit (equivalent to 1,000,000 microcredits) from the public balance to a recipient address.
//!
//! ```
//! use aleo_agent::account::Account;
//! use aleo_agent::agent::{Agent, TransferArgs, TransferType};
//! use aleo_agent::{Address, MICROCREDITS};
//! use anyhow::Result;
//! use std::str::FromStr;
//!
//! // recipient address format: aleo1...
//! fn transfer_public_balance(recipient_address : &str) -> Result<()> {
//!     // private key format: APrivateKey1zkp...
//!     let private_key = "YOUR PRIVATE KEY";
//!     // build an account using the private key
//!     let account = Account::from_private_key(private_key)?;
//!     // build an agent using the account
//!     let agent = Agent::builder().with_account(account).build();
//!     
//!     let public_balance = agent.get_public_balance()?;
//!     println!("Public Balance : {}", public_balance);
//!     
//!     let recipient_address = Address::from_str(recipient_address).expect("Invalid recipient address");
//!     // transfer 1 credit to recipient_address
//!     let transfer_args = TransferArgs::from(
//!         MICROCREDITS, // transfer 1 credit
//!         recipient_address,
//!         1, // priority fee
//!         None, // no record, using public balance
//!         TransferType::Public, // transfer 1 credit using public balance
//!     );
//!     let tx_hash = agent.transfer(transfer_args)?;
//!     println!("Transfer tx hash: {}", tx_hash);
//!     
//!     Ok(())
//! }
//! ```
//! For more information about the Agent interface used in this example, see the examples in the `agent` module.
//!
//! ## References
//! For an introduction to the Aleo Network and the Aleo Program,
//! see the following resources:
//!
//! - [SnarkOS](https://github.com/AleoHQ/snarkOS)
//! - [SnarkVM](https://github.com/AleoHQ/snarkVM)
//! - [Aleo Developer Guide](https://developer.aleo.org/getting_started/)

pub use snarkvm::console::{
    network::Testnet3,
    prelude::Uniform,
    program::{Entry, Literal, Network, Record},
};
pub use snarkvm::ledger::store::helpers::memory::BlockMemory;

pub mod account;
pub mod agent;
pub mod builder;
pub mod chain;
pub mod deploy;
pub mod program;

// GLOBAL DECLARATIONS
pub type CurrentNetwork = Testnet3;
pub type TransactionID = <CurrentNetwork as Network>::TransactionID;
pub type CiphertextRecord = Record<CurrentNetwork, Ciphertext>;
pub type PlaintextRecord = Record<CurrentNetwork, Plaintext>;
pub type BlockHash = <CurrentNetwork as Network>::BlockHash;
pub type TransitionID = <CurrentNetwork as Network>::TransitionID;
pub type ProgramID = snarkvm::console::program::ProgramID<CurrentNetwork>;
pub type Identifier = snarkvm::console::program::Identifier<CurrentNetwork>;
pub type Value = snarkvm::console::program::Value<CurrentNetwork>;
pub type Field = snarkvm::console::types::Field<CurrentNetwork>;
pub type Ciphertext = snarkvm::console::program::Ciphertext<CurrentNetwork>;
pub type Plaintext = snarkvm::console::program::Plaintext<CurrentNetwork>;
pub type PrivateKey = snarkvm::console::account::PrivateKey<CurrentNetwork>;
pub type ViewKey = snarkvm::console::account::ViewKey<CurrentNetwork>;
pub type Signature = snarkvm::console::account::Signature<CurrentNetwork>;
pub type Address = snarkvm::console::account::Address<CurrentNetwork>;
pub type Group = snarkvm::console::account::Group<CurrentNetwork>;
pub type Query = snarkvm::ledger::query::Query<CurrentNetwork, BlockMemory<CurrentNetwork>>;
pub type Block = snarkvm::ledger::Block<CurrentNetwork>;
pub type Transaction = snarkvm::ledger::Transaction<CurrentNetwork>;
pub type ConfirmedTransaction = snarkvm::ledger::ConfirmedTransaction<CurrentNetwork>;
pub type Transactions = snarkvm::ledger::Transactions<CurrentNetwork>;
pub type ConsensusMemory = snarkvm::ledger::store::helpers::memory::ConsensusMemory<CurrentNetwork>;
pub type ConsensusStore = snarkvm::ledger::store::ConsensusStore<CurrentNetwork, ConsensusMemory>;
pub type VM = snarkvm::synthesizer::VM<CurrentNetwork, ConsensusMemory>;
pub type Program = snarkvm::synthesizer::Program<CurrentNetwork>;
pub type Package = snarkvm::package::Package<CurrentNetwork>;

pub const DEFAULT_BASE_URL: &str = "https://api.explorer.aleo.org/v1";
pub const DEFAULT_TESTNET: &str = "testnet3";
pub const MAINNET: &str = "mainnet";
pub const MICROCREDITS: u64 = 1_000_000; // 1 credit = 1_000_000 microcredits
