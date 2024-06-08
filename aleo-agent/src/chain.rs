//! Node APIs
use anyhow::{bail, Result};

use crate::agent::Agent;

use super::*;

// chain
impl Agent {
    /// Retrieves the latest block height from the network.
    ///
    /// # Returns
    /// The `Ok` variant wraps the latest block height as `u32`.
    pub fn get_latest_block_height(&self) -> Result<u32> {
        let url = format!("{}/{}/block/height/latest", self.base_url(), self.network());
        match self.client().get(&url).call()?.into_json() {
            Ok(height) => Ok(height),
            Err(error) => bail!("Failed to parse the latest block height: {error}"),
        }
    }

    /// Retrieves the latest block hash from the network.
    ///
    /// # Returns
    /// The `Ok` variant wraps the latest block hash as `BlockHash`.
    pub fn get_latest_block_hash(&self) -> Result<BlockHash> {
        let url = format!("{}/{}/block/hash/latest", self.base_url(), self.network());
        match self.client().get(&url).call()?.into_json() {
            Ok(hash) => Ok(hash),
            Err(error) => bail!("Failed to parse the latest block hash: {error}"),
        }
    }

    /// Retrieves the latest block from the network.
    ///
    /// # Returns
    /// The `Ok` variant wraps the latest block as `Block`.
    pub fn get_latest_block(&self) -> Result<Block> {
        let url = format!("{}/{}/latest/block/height", self.base_url(), self.network());
        match self.client().get(&url).call()?.into_json() {
            Ok(block) => Ok(block),
            Err(error) => bail!("Failed to parse the latest block: {error}"),
        }
    }

    /// Retrieves the block of a specific height from the network.
    ///
    /// # Arguments
    /// * `height` - The height of the block to retrieve.
    ///
    /// # Returns
    /// The `Ok` variant wraps the block of the specific height as `Block`.
    pub fn get_block_of_height(&self, height: u32) -> Result<Block> {
        let url = format!("{}/{}/block/{height}", self.base_url(), self.network());
        match self.client().get(&url).call()?.into_json() {
            Ok(block) => Ok(block),
            Err(error) => bail!("Failed to parse block {height}: {error}"),
        }
    }

    /// Retrieves the transactions of a block of a specific height from the network.
    ///
    /// # Arguments
    /// * `height` - The height of the block.
    ///
    /// # Returns
    /// The `Ok` variant wraps the transactions of the block of the specific height as `Transactions`.
    pub fn get_transactions_of_height(&self, height: u32) -> Result<Transactions> {
        let url = format!(
            "{}/{}/block/{height}/transactions",
            self.base_url(),
            self.network()
        );
        match self.client().get(&url).call()?.into_json() {
            Ok(block) => Ok(block),
            Err(error) => bail!("Failed to parse block {height}: {error}"),
        }
    }

    /// Retrieves a range of blocks from the network.
    ///
    /// # Arguments
    /// * `start_height` - The starting height of the range of blocks to retrieve.
    /// * `end_height` - The ending height of the range of blocks to retrieve.
    /// *  end_height - start_height must be less than or equal to 50.
    ///
    /// # Returns
    /// The `Ok` variant wraps a vector of `Block`.
    pub fn get_blocks_in_range(&self, start_height: u32, end_height: u32) -> Result<Vec<Block>> {
        if start_height >= end_height {
            bail!("Start height must be less than end height");
        }

        if end_height - start_height > 50 {
            bail!("The range of blocks must be less than 50");
        }

        let url = format!(
            "{}/{}/blocks?start={start_height}&end={end_height}",
            self.base_url(),
            self.network()
        );
        match self.client().get(&url).call()?.into_json() {
            Ok(blocks) => Ok(blocks),
            Err(error) => {
                bail!("Failed to parse blocks {start_height} (inclusive) to {end_height} (exclusive): {error}")
            }
        }
    }

    /// Retrieves a transaction by its transaction id from the network.
    ///
    /// # Arguments
    /// * `transaction_id` - The id of the transaction to retrieve.
    ///
    /// # Returns
    /// The `Ok` variant wraps the transaction as `Transaction`.
    pub fn get_transaction(&self, transaction_id: &str) -> Result<Transaction> {
        let url = format!(
            "{}/{}/transaction/{}",
            self.base_url(),
            self.network(),
            transaction_id
        ).replace('"', "");
        match self.client().get(&url).call()?.into_json() {
            Ok(transaction) => Ok(transaction),
            Err(error) => bail!("Failed to parse transaction '{transaction_id}': {error}"),
        }
    }

    /// Retrieves the confirmed transaction for a given transaction id from the network.
    ///
    /// # Arguments
    /// * `transaction_id` - The id of the transaction to retrieve.
    ///
    /// # Returns
    /// The `Ok` variant wraps the confirmed transaction as `ConfirmedTransaction`.
    pub fn get_confirmed_transaction(&self, transaction_id: &str) -> Result<ConfirmedTransaction> {
        let url = format!(
            "{}/{}/transaction/confirmed/{}",
            self.base_url(),
            self.network(),
            transaction_id
        ).replace('"', "");
        match self.client().get(&url).call()?.into_json() {
            Ok(transaction) => Ok(transaction),
            Err(error) => bail!("Failed to parse transaction '{transaction_id}': {error}"),
        }
    }

    /// Retrieves the pending transactions currently in the mempool from the network.
    ///
    /// # Returns
    /// The `Ok` variant wraps the pending transactions as a vector of `Transaction`.
    // pub fn get_mempool_transactions(&self) -> Result<Vec<Transaction>> {
    //     let url = format!(
    //         "{}/{}/memoryPool/transactions",
    //         self.base_url(),
    //         self.network()
    //     );
    //     match self.client().get(&url).call()?.into_json() {
    //         Ok(transactions) => Ok(transactions),
    //         Err(error) => bail!("Failed to parse memory pool transactions: {error}"),
    //     }
    // }

    /// Broadcasts a transaction to the Aleo network.
    ///
    /// # Arguments
    /// * `transaction` - The transaction to broadcast.
    ///
    /// # Returns
    /// The `Ok` variant wraps the Transaction ID from the network as a `String`.
    pub fn broadcast_transaction(&self, transaction: &Transaction) -> Result<String> {
        let url = format!(
            "{}/{}/transaction/broadcast",
            self.base_url(),
            self.network()
        );
        match self.client().post(&url).send_json(transaction) {
            Ok(response) => match response.into_string() {
                Ok(success_response) => {
                    Ok(success_response)
                }
                Err(error) => bail!("❌ Transaction response was malformed {}", error),
            },
            Err(error) => {
                let error_message = match error {
                    ureq::Error::Status(code, response) => {
                        format!("(status code {code}: {:?})", response.into_string()?)
                    }
                    ureq::Error::Transport(err) => format!("({err})"),
                };

                match transaction {
                    Transaction::Deploy(..) => {
                        bail!("❌ Failed to deploy program to {}: {}", &url, error_message)
                    }
                    Transaction::Execute(..) => {
                        bail!(
                            "❌ Failed to broadcast execution to {}: {}",
                            &url,
                            error_message
                        )
                    }
                    Transaction::Fee(..) => {
                        bail!(
                            "❌ Failed to broadcast fee execution to {}: {}",
                            &url,
                            error_message
                        )
                    }
                }
            }
        }
    }

    /// Returns the block hash that contains the given transaction ID.
    ///
    /// # Arguments
    /// * `transaction_id` - The id of the transaction to find the block hash for.
    ///
    /// # Returns
    /// The `Ok` variant wraps the block hash as `BlockHash`.
    pub fn find_block_hash_by_transaction_id(
        &self,
        transaction_id: &TransactionID,
    ) -> Result<BlockHash> {
        let url = format!(
            "{}/{}/find/blockHash/{}",
            self.base_url(),
            self.network(),
            transaction_id
        ).replace('"', "");
        match self.client().get(&url).call()?.into_json() {
            Ok(hash) => Ok(hash),
            Err(error) => bail!("Failed to parse block hash: {error}"),
        }
    }

    /// Retrieves the transition ID that contains the given `input ID` or `output ID` from the network.
    ///
    /// # Arguments
    /// * `input_or_output_id` - The `input ID` or `output ID` to find the transition ID for.
    ///
    /// # Returns
    /// The `Ok` variant wraps the transition ID as `TransitionID`.
    pub fn find_transition_id_by_input_or_output_id(
        &self,
        input_or_output_id: Field,
    ) -> Result<TransitionID> {
        let url = format!(
            "{}/{}/find/transitionID/{input_or_output_id}",
            self.base_url(),
            self.network()
        );
        match self.client().get(&url).call()?.into_json() {
            Ok(transition_id) => Ok(transition_id),
            Err(error) => bail!("Failed to parse transition ID: {error}"),
        }
    }
}

#[cfg(test)]
mod test{
    use std::str::FromStr;
    use snarkvm::prelude::AleoID;
    use super::*;

    #[test]
    fn test_find_transition_id_by_public_input_id() {
        let agent = Agent::default();
        let input_id = Field::from_str("442821668769577970144612761629986410250375075037739392584772366002083927285field").unwrap();
        let res = agent
            .find_transition_id_by_input_or_output_id(input_id)
            .expect("Failed to find transition ID by input ID");
        assert_eq!(res, AleoID::from_str("au16zlg0gwj2wnrxgq8699vdrc46s4a6eefg6frd5skr5e3fr8j2u8q4cs9wz").unwrap())
    }

    #[test]
    fn test_find_transition_id_by_output_id() {
        let agent = Agent::default();
        let output_id = Field::from_str("4718225685615532558993353175858434048183497319430064832948717582958793823285field").unwrap();
        let res = agent
            .find_transition_id_by_input_or_output_id(output_id)
            .expect("Failed to find transition ID by input ID");
        assert_eq!(res, AleoID::from_str("au16zlg0gwj2wnrxgq8699vdrc46s4a6eefg6frd5skr5e3fr8j2u8q4cs9wz").unwrap())
    }

    #[test]
    fn test_find_block_hash_by_transaction_id() {
        let agent = Agent::default();
        let transaction_id = TransactionID::from_str("at1z6ydwyklzlhe4xm8uferf9uevsynxjfkqmgcxps6rjl4x737zq8qr4s3rv").unwrap();
        let res = agent
            .find_block_hash_by_transaction_id(&transaction_id)
            .expect("Failed to find block hash by transaction ID");
        assert_eq!(res, BlockHash::from_str("ab1mmn6rntv2qhyz8qdsjdreevsax867ha2k8hysgflz9m03c06m5yqsmpke3").unwrap())
    }

    // #[test]
    // fn test_get_mempool_txs(){
    //     let agent = Agent::default();
    //     let res = agent.get_mempool_transactions().expect("Failed to get mempool transactions");
    //     println!("Mempool Transactions: {:?}", res);
    // }

    #[test]
    fn test_get_transaction_by_id(){
        let agent = Agent::default();
        let transaction_id = "at1z6ydwyklzlhe4xm8uferf9uevsynxjfkqmgcxps6rjl4x737zq8qr4s3rv";
        let res = agent.get_transaction(transaction_id).expect("Failed to get transaction by id");
        assert_eq!(res.id(), TransactionID::from_str(transaction_id).unwrap())
    }

    #[test]
    fn test_get_confirmed_transaction_by_id() {
        let agent = Agent::default();
        let transaction_id = "at1z6ydwyklzlhe4xm8uferf9uevsynxjfkqmgcxps6rjl4x737zq8qr4s3rv";
        let res = agent.get_confirmed_transaction(transaction_id).expect("Failed to get confirmed transaction by id");
        assert_eq!(res.id(), TransactionID::from_str(transaction_id).unwrap())
    }
}