//! The main Agent module. Contains the [Agent] types and all associated structures

use crate::account::Account;
use crate::builder::AgentBuilder;
use crate::program::ProgramManager;
use anyhow::{bail, ensure, Result};
use snarkvm::circuit::prelude::num_traits::ToPrimitive;
use std::fmt;
use std::ops::Range;
use std::str::FromStr;

use crate::{
    Address, CiphertextRecord, ConsensusStore, CurrentNetwork, Entry, Field, Identifier, Literal,
    Plaintext, PlaintextRecord, ProgramID, Query, Transaction, Value, DEFAULT_BASE_URL,
    DEFAULT_TESTNET, VM,
};

#[derive(Clone)]
pub struct Agent {
    client: ureq::Agent,
    base_url: String,
    network: String,
    account: Account,
}

impl Default for Agent {
    fn default() -> Agent {
        Self {
            client: ureq::Agent::new(),
            account: Account::default(),
            base_url: DEFAULT_BASE_URL.to_string(),
            network: DEFAULT_TESTNET.to_string(),
        }
    }
}

impl Agent {
    pub fn builder() -> AgentBuilder {
        AgentBuilder::default()
    }

    pub fn new(base_url: String, network: String, account: Account) -> Agent {
        Agent {
            client: ureq::Agent::new(),
            base_url,
            network,
            account,
        }
    }

    pub fn program(&self, program_id: &str) -> Result<ProgramManager> {
        let program_id = ProgramID::from_str(program_id)?;
        Ok(ProgramManager::new(self, program_id))
    }

    pub fn account(&self) -> &Account {
        &self.account
    }

    pub fn base_url(&self) -> &String {
        &self.base_url
    }

    pub fn client(&self) -> &ureq::Agent {
        &self.client
    }

    pub fn network(&self) -> &String {
        &self.network
    }

    pub fn set_url(&mut self, url: &str) {
        self.base_url = url.to_string();
    }

    pub fn set_network(&mut self, network: &str) {
        self.network = network.to_string();
    }

    pub fn set_account(&mut self, account: Account) {
        self.account = account;
    }

    pub fn local_testnet(&mut self, port: &str) {
        self.network = DEFAULT_TESTNET.to_string();
        self.base_url = format!("http://0.0.0.0:{}", port);
    }
}

impl Agent {
    /// Decrypts a ciphertext record to a plaintext record using the agent's view key.
    ///
    /// # Arguments
    /// * `ciphertext_record` - The ciphertext record to decrypt.
    ///
    /// # Returns
    /// A `Result` which is:
    /// * a `PlaintextRecord` - The decrypted plaintext record.
    /// * an `Error` - If there was an issue decrypting the record.
    ///
    /// # Example
    /// ```ignore
    /// use std::str::FromStr;
    /// use aleo_agent::agent::Agent;
    /// use aleo_agent::CiphertextRecord;
    /// let agent = Agent::default();
    /// let ciphertext_record = CiphertextRecord::from_str( "CIPHERTEXT RECORD").expect("Failed to parse ciphertext record");
    /// let plaintext_record = agent.decrypt_ciphertext_record(&ciphertext_record);
    /// ```
    pub fn decrypt_ciphertext_record(
        &self,
        ciphertext_record: &CiphertextRecord,
    ) -> Result<PlaintextRecord> {
        let view_key = self.account().view_key();
        ciphertext_record.decrypt(view_key)
    }

    /// Finds unspent records on chain.
    ///
    /// # Arguments
    /// * `block_heights` - The range of block heights to search for unspent records.
    /// * `max_gates` - The minimum threshold microcredits for the sum of balances collected from records
    ///
    /// # Returns
    /// The `Ok` variant wraps the unspent records as a vector of tuples of `(Field, PlaintextRecord)`.
    ///
    /// # Example
    /// ```
    /// use aleo_agent::agent::Agent;
    /// use aleo_agent::{MICROCREDITS, PlaintextRecord};
    /// let agent = Agent::default();
    /// let gate = 10 * MICROCREDITS;
    ///
    /// // Get unspent records with a minimum of 10 credits in the range of blocks 0 to 100
    /// let res = agent.get_unspent_records(0..100, Some(gate)).expect("Failed to get unspent records");
    /// let records = res
    ///  .iter().filter_map(|(_, record)| Some(record.cloned()) )
    ///  .collect::<Vec<PlaintextRecord>>();
    /// ```
    pub fn get_unspent_records(
        &self,
        block_heights: Range<u32>,
        max_gates: Option<u64>, // microcredits
    ) -> Result<Vec<(Field, PlaintextRecord)>> {
        ensure!(
            block_heights.start < block_heights.end,
            "The start block height must be less than the end block height"
        );

        let private_key = self.account().private_key();
        let view_key = self.account().view_key();
        let address_x_coordinate = self.account().address().to_x_coordinate();

        let step_size = 49;

        // Initialize a vector for the records.
        let mut records = vec![];

        let mut total_gates = 0u64;
        let mut end_height = block_heights.end;
        let mut start_height = block_heights.end.saturating_sub(step_size);

        for _ in (block_heights.start..block_heights.end).step_by(step_size as usize) {
            println!(
                "Searching blocks {} to {} for records...",
                start_height, end_height
            );
            // Get blocks
            let _records = self
                .get_blocks_in_range(start_height, end_height)?
                .into_iter()
                .flat_map(|block| block.into_records())
                .filter_map(|(commitment, record)| {
                    if record.is_owner_with_address_x_coordinate(view_key, &address_x_coordinate) {
                        let sn = PlaintextRecord::serial_number(*private_key, commitment).ok()?;
                        if self.find_transition_id_by_input_or_output_id(sn).is_err() {
                            if let Ok(record) = record.decrypt(view_key) {
                                total_gates += record.microcredits().unwrap_or(0);
                                return Some((commitment, record));
                            }
                        }
                    };
                    None
                });

            // Filter the records by the view key.
            records.extend(_records);

            // If a maximum number of gates is specified, stop searching when the total gates
            // use the specified limit
            if max_gates.is_some() && total_gates >= max_gates.unwrap() {
                break;
            }

            // Search in reverse order from the latest block to the earliest block
            end_height = start_height;
            start_height = start_height.saturating_sub(step_size);
            if start_height < block_heights.start {
                start_height = block_heights.start
            };
        }
        Ok(records)
    }

    /// Scans the chain for all records matching the address of agent.
    ///
    /// # Return
    /// A `Result` which is:
    /// * a `Vec<(Field, PlaintextRecord)>` - The records that match the view key.
    /// * an `Error` - If there was an issue scanning the ledger.
    ///
    /// # Example
    /// ```ignore
    ///     use aleo_agent::agent::Agent;
    ///     let agent = Agent::default();
    ///     let end_height = agent.get_latest_block_height().unwrap();
    ///     let start_height = end_height - 50; // You can arbitrarily specify the {start block}
    ///     let records = agent.scan_records(start_height..end_height, None);
    ///     match records {
    ///         Ok(records) => {
    ///             println!("Records:\n{records:#?}");
    ///         }
    ///         Err(e) => {
    ///             eprintln!("Failed to get records : {e}");
    ///         }
    ///     }
    /// ```
    pub fn scan_records(
        &self,
        block_heights: Range<u32>,
        max_records: Option<usize>,
    ) -> Result<Vec<(Field, PlaintextRecord)>> {
        // Compute the x-coordinate of the address.
        let address_x_coordinate = self.account().address().to_x_coordinate();

        // Prepare the starting block height, by rounding down to the nearest step of 50.
        let start_block_height = block_heights.start - (block_heights.start % 50);
        // Prepare the ending block height, by rounding up to the nearest step of 50.
        let end_block_height = block_heights.end + (50 - (block_heights.end % 50));

        // Initialize a vector for the records.
        let mut records = Vec::new();

        for start_height in (start_block_height..end_block_height).step_by(50) {
            println!(
                "Searching blocks {} to {} for records...",
                start_height, end_block_height
            );
            if start_height >= block_heights.end {
                break;
            }
            let end = start_height + 50;
            let end_height = if end > block_heights.end {
                block_heights.end
            } else {
                end
            };

            let view_key = self.account().view_key();
            // Filter the records by the view key.
            let _records = self
                .get_blocks_in_range(start_height, end_height)?
                .into_iter()
                .flat_map(|block| block.into_records())
                .filter_map(|(commitment, record)| {
                    if record.is_owner_with_address_x_coordinate(view_key, &address_x_coordinate) {
                        Some((
                            commitment,
                            record.decrypt(view_key).expect("Failed to decrypt records"),
                        ))
                    } else {
                        None
                    }
                })
                .collect::<Vec<(Field, PlaintextRecord)>>();

            records.extend(_records);

            if records.len() >= max_records.unwrap_or(usize::MAX) {
                break;
            }
        }

        Ok(records)
    }
}

impl Agent {
    /// Fetch the public balance in microcredits associated with the address.
    ///
    /// # Returns
    /// A `Result` which is:
    /// * a `u64` - The public balance in microcredits associated with the address.
    /// * an `Error` - If there was an issue fetching the public balance.
    pub fn get_public_balance(&self) -> Result<u64> {
        let credits = ProgramID::from_str("credits.aleo")?;
        let account_mapping = Identifier::from_str("account")?;
        let url = format!(
            "{}/{}/program/{}/mapping/{}/{}",
            self.base_url(),
            self.network(),
            credits,
            account_mapping,
            self.account().address()
        );
        let response = self.client().get(&url).call()?;
        Ok(response
            .into_json::<Option<Value>>()?
            .and_then(|value| match value {
                //Value::Plaintext(Plaintext::Literal(Literal::U64(amount), _))
                Value::Plaintext(Plaintext::Literal(Literal::U64(amount), _)) => {
                    Some(amount.to_u64().unwrap())
                }
                _ => None,
            })
            .unwrap_or_default())
    }

    /// Fetches the transactions associated with the agent's account.
    ///
    /// # Returns
    /// A `Result` which is:
    /// * a `Vec<Transaction>` - The transactions associated with the agent's account.
    /// * an `Error` - If there was an issue fetching the transactions.
    pub fn get_transactions(&self) -> Result<Vec<Transaction>> {
        let url = format!(
            "{}/{}/address/{}",
            self.base_url(),
            self.network(),
            self.account().address()
        );
        match self.client().get(&url).call()?.into_json() {
            Ok(transaction) => Ok(transaction),
            Err(error) => bail!("Failed to get account transactions : {error}"),
        }
    }

    /// Executes a transfer to the specified recipient_address with the specified amount and fee.
    ///
    /// # Arguments
    /// * `amount` - The amount to be transferred.
    /// * `fee` - The fee for the transfer.
    /// * `recipient_address` - The address of the recipient.
    /// * `transfer_type` - The type of transfer.
    /// * `amount_record` - An optional record of the amount.
    /// * `fee_record` - An optional record of the fee.
    ///
    /// # Returns
    /// A `Result` which is:
    /// * a `String` - The transaction hash .
    /// * an `Error` - If there was an issue executing the transfer.
    /// Executes a transfer to the specified recipient_address with the specified amount and fee.
    /// Specify 0 for no fee.
    ///
    /// # Example
    /// ```ignore
    /// use std::str::FromStr;
    /// use aleo_agent::Address;
    /// use aleo_agent::agent::{Agent, TransferArgs, TransferType};
    /// let agent = Agent::default();
    /// // just use for test
    /// let recipient_address = Address::zero();
    /// let amount = 100;
    /// let priority_fee = 0;
    ///
    /// // Public transfer 100 microcredits to the recipient address with 0 priority fee
    /// let transfer_args = TransferArgs::from(amount, recipient_address, priority_fee, None, TransferType::Public);
    /// let transfer_result = agent.transfer(transfer_args);
    /// ```
    pub fn transfer(&self, args: TransferArgs) -> Result<String> {
        match &(args.transfer_type) {
            TransferType::Private(from_record) | TransferType::PrivateToPublic(from_record) => {
                ensure!(
                    from_record.microcredits()? >= args.amount,
                    "Credits in amount record must greater than transfer amount specified"
                );
            }
            _ => {}
        }

        if let Some(fee_record) = args.fee_record.as_ref() {
            ensure!(
                fee_record.microcredits()? >= args.priority_fee,
                "Credits in fee record must greater than fee specified"
            );
        }

        let inputs = args.to_inputs();
        let transfer_function = args.transfer_type.to_string();
        let rng = &mut rand::thread_rng();
        // Initialize a VM
        let store = ConsensusStore::open(None)?;
        let vm = VM::from(store)?;
        // Specify the network state query
        let query = Query::from(self.base_url().clone());
        // Create a new transaction.
        let execution = vm.execute(
            self.account().private_key(),
            ("credits.aleo", transfer_function),
            inputs.iter(),
            args.fee_record,
            args.priority_fee,
            Some(query),
            rng,
        )?;
        self.broadcast_transaction(&execution)
    }
}

/// A trait providing convenient methods for accessing the amount of Aleo present in a record
pub trait Credits {
    /// Get the amount of credits in the record if the record possesses Aleo credits
    fn credits(&self) -> Result<f64> {
        Ok(self.microcredits()? as f64 / 1_000_000.0)
    }

    /// Get the amount of microcredits in the record if the record possesses Aleo credits
    fn microcredits(&self) -> Result<u64>;
}

impl Credits for PlaintextRecord {
    fn microcredits(&self) -> Result<u64> {
        let amount = match self.find(&[Identifier::from_str("microcredits")?])? {
            Entry::Private(Plaintext::Literal(Literal::<CurrentNetwork>::U64(amount), _)) => amount,
            _ => bail!("The record provided does not contain a microcredits field"),
        };
        Ok(*amount)
    }
}

#[derive(Clone, Debug)]
pub enum TransferType {
    // param: from record plaintext
    Private(PlaintextRecord),
    // param: from record plaintext
    PrivateToPublic(PlaintextRecord),
    Public,
    PublicToPrivate,
}

impl fmt::Display for TransferType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TransferType::Private(_) => write!(f, "transfer_private"),
            TransferType::PrivateToPublic(_) => write!(f, "transfer_private_to_public"),
            TransferType::Public => write!(f, "transfer_public"),
            TransferType::PublicToPrivate => write!(f, "transfer_public_to_private"),
        }
    }
}

/// Arguments for a transfer.
/// amount, fee, recipient address, from record
#[derive(Clone, Debug)]
pub struct TransferArgs {
    amount: u64,       // microcredits
    priority_fee: u64, // microcredits
    recipient_address: Address,
    transfer_type: TransferType,
    fee_record: Option<PlaintextRecord>,
}

impl TransferArgs {
    /// Create a new transfer argument.
    ///
    /// # Arguments
    /// * `amount` - The amount to be transferred.
    /// * `recipient_address` - The address of the recipient.
    /// * `priority_fee` - The fee for the transfer.
    /// * `fee_record` - An optional record of the fee.
    /// * `transfer_type` - The type of transfer.
    ///
    /// # Returns
    /// A `TransferArgs` - The transfer arguments.
    ///
    /// # Example
    /// ```
    /// use std::str::FromStr;
    /// use aleo_agent::{Address, MICROCREDITS};
    /// use aleo_agent::agent::{TransferArgs, TransferType};
    /// let recipient_address = Address::zero();
    /// let amount = 10 * MICROCREDITS; // 10 credit
    /// let priority_fee = 0;
    /// let transfer_args = TransferArgs::from(amount, recipient_address, priority_fee, None, TransferType::Public);
    /// ```
    pub fn from(
        amount: u64,
        recipient_address: Address,
        priority_fee: u64,
        fee_record: Option<PlaintextRecord>,
        transfer_type: TransferType,
    ) -> Self {
        Self {
            amount,
            priority_fee,
            recipient_address,
            transfer_type,
            fee_record,
        }
    }

    /// Convert the transfer arguments to a vector of values.
    ///
    /// # Returns
    /// A `Vec<Value>` - The transfer arguments as a vector of values.
    pub fn to_inputs(&self) -> Vec<Value> {
        match &(self.transfer_type) {
            TransferType::Private(from_record) | TransferType::PrivateToPublic(from_record) => {
                vec![
                    Value::Record(from_record.clone()),
                    Value::from_str(&self.recipient_address.to_string()).unwrap(),
                    Value::from_str(&format!("{}u64", self.amount)).unwrap(),
                ]
            }
            _ => {
                vec![
                    Value::from_str(&self.recipient_address.to_string()).unwrap(),
                    Value::from_str(&format!("{}u64", self.amount)).unwrap(),
                ]
            }
        }
    }
}
