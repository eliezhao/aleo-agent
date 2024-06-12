//! Tools for executing, and managing programs on the Aleo network
//! Program management object for loading programs for building, execution, and deployment
//!
//! This object is meant to be a software abstraction that can be consumed by software like
//! CLI tools, IDE plugins, Server-side stack components and other software that needs to
//! interact with the Aleo network.

use std::cmp::min;
use std::ops::Range;
use std::path::PathBuf;
use std::str::FromStr;

use crate::agent::Agent;
use anyhow::{anyhow, bail, ensure, Error, Result};
use indexmap::IndexMap;

use super::*;

#[derive(Clone)]
pub struct ProgramManager<'agent> {
    agent: &'agent Agent,
    program_id: ProgramID,
}

impl<'agent> ProgramManager<'agent> {
    /// Creates a new Program Manager with an agent and a particular ProgramID.
    pub fn new(agent: &'agent Agent, program_id: ProgramID) -> Self {
        Self { agent, program_id }
    }

    pub fn program_id(&self) -> &ProgramID {
        &self.program_id
    }

    pub fn agent(&self) -> &Agent {
        self.agent
    }
}

// execution functions
impl<'agent> ProgramManager<'agent> {
    /// Execute a program function on the Aleo Network.
    ///
    /// To run this function successfully, the program must already be deployed on the Aleo Network
    ///
    /// # Arguments
    /// * `function` - The function to execute
    /// * `inputs` - The inputs to the function
    /// * `priority_fee` - The priority fee to pay for the transaction
    /// * `fee_record` - The plaintext record to pay for the transaction fee. If None, the fee will be paid through the account's public balance
    ///
    /// # Returns
    /// The transaction ID of the execution transaction
    ///
    /// # Example
    /// ```ignore
    /// use aleo_agent::agent::Agent;
    /// use aleo_agent::program::ProgramManager;
    /// let pm = Agent::default().program("xxx.aleo");
    ///
    /// // Execute the main function of the xxx.aleo program with inputs 1, 2, 3; priority fee 100; and no fee record
    /// // The fee will be paid through account's public balance
    /// let tx_id = pm.execute_program("main", vec![1, 2, 3].into_iter(), 100, None).expect("Failed to execute program");
    /// ```
    pub fn execute_program(
        &self,
        function: &str,
        inputs: impl ExactSizeIterator<Item = impl TryInto<Value>>,
        priority_fee: u64,
        fee_record: Option<PlaintextRecord>,
    ) -> Result<String> {
        // Check program and function have valid names
        let function_id: Identifier =
            Identifier::from_str(function).map_err(|_| anyhow!("Invalid function name"))?;

        // Get the program from chain, error if it doesn't exist
        let program = Self::get_program_from_chain(self.program_id())?;

        // Initialize an RNG and query object for the transaction
        let rng = &mut rand::thread_rng();
        let query = Query::from(self.agent().base_url());

        let vm = Self::initialize_vm(&program)?;

        let transaction = vm.execute(
            self.agent().account().private_key(),
            (program.id(), function_id),
            inputs,
            fee_record,
            priority_fee,
            Some(query),
            rng,
        )?;

        // Broadcast the execution transaction to the network
        self.agent().broadcast_transaction(&transaction)
    }

    /// Execute a program function on the Aleo Network with a priority fee and no fee record
    ///
    /// # Arguments
    /// * `block_heights` - The range of block heights to search for records
    /// * `unspent_only` - Whether to return only unspent records : true for unspent records, false for all records
    ///
    /// # Returns
    /// A vector of records that match the search criteria
    ///
    /// # Example
    /// ```ignore
    /// use aleo_agent::agent::Agent;
    /// use aleo_agent::program::ProgramManager;
    /// let pm = Agent::default().program("xxx.aleo");
    ///
    /// // Get the unspent records of the first 100 blocks for the program
    /// let records = pm.get_program_records(0..100, true).expect("Failed to get program records");
    /// ```
    pub fn get_program_records(
        &self,
        block_heights: Range<u32>,
        unspent_only: bool,
    ) -> Result<Vec<(Field, CiphertextRecord)>> {
        let private_key = self.agent().account().private_key();
        // Prepare the view key.
        let view_key = self.agent().account().view_key();
        // Compute the x-coordinate of the address.
        let address_x_coordinate = view_key.to_address().to_x_coordinate();

        // Prepare the starting block height, by rounding down to the nearest step of 50.
        let start_block_height = block_heights.start - (block_heights.start % 50);
        // Prepare the ending block height, by rounding up to the nearest step of 50.
        let end_block_height = block_heights.end + (50 - (block_heights.end % 50));

        // Initialize a vector for the records.
        let mut records = Vec::new();

        for start_height in (start_block_height..end_block_height).step_by(50) {
            if start_height >= block_heights.end {
                break;
            }
            let end_height = min(start_height + 50, block_heights.end);

            let _records = self
                .agent()
                .get_blocks_in_range(start_height, end_height)?
                .into_iter()
                .flat_map(|block| block.into_transitions())
                .filter(|transition| transition.program_id().eq(self.program_id()))
                .flat_map(|transition| transition.into_records())
                .filter_map(|(commitment, record)| {
                    if record.is_owner_with_address_x_coordinate(view_key, &address_x_coordinate) {
                        if unspent_only {
                            let sn =
                                CiphertextRecord::serial_number(*private_key, commitment).ok()?;
                            if self
                                .agent()
                                .find_transition_id_by_input_or_output_id(sn)
                                .is_err()
                            {
                                return Some((commitment, record));
                            }
                        } else {
                            return Some((commitment, record));
                        }
                    };
                    None
                });
            records.extend(_records);
        }

        Ok(records)
    }

    /// Get the current value of a mapping given a specific program, mapping name, and mapping key
    ///
    /// # Arguments
    /// * `mapping_name` - The name of the mapping to query
    /// * `key` - The key to query the mapping with
    ///
    /// # Returns
    /// The value of the mapping at the given key
    pub fn get_mapping_value(
        &self,
        mapping_name: impl TryInto<Identifier>,
        key: impl TryInto<Plaintext>,
    ) -> Result<Value> {
        // Prepare the mapping name.
        let mapping_name = mapping_name
            .try_into()
            .map_err(|_| anyhow!("Invalid mapping name"))?;
        // Prepare the key.
        let key = key.try_into().map_err(|_| anyhow!("Invalid key"))?;
        let program_id = self.program_id();
        // Perform the request.
        let url = format!(
            "{}/{}/program/{}/mapping/{mapping_name}/{key}",
            self.agent().base_url(),
            self.agent().network(),
            program_id.to_string(),
        );
        match self.agent().client().get(&url).call()?.into_json() {
            Ok(transition_id) => Ok(transition_id),
            Err(error) => bail!("Failed to parse transition ID: {error}"),
        }
    }

    /// Get all mappings associated with a program.
    pub fn get_program_mappings(&self) -> Result<Vec<Identifier>> {
        // Prepare the program ID.
        let program_id = self.program_id();
        // Perform the request.
        let url = format!(
            "{}/{}/program/{}/mappings",
            self.agent().base_url(),
            self.agent().network(),
            program_id.to_string()
        );
        match self.agent().client().get(&url).call()?.into_json() {
            Ok(program_mappings) => Ok(program_mappings),
            Err(error) => bail!("Failed to parse program {program_id}: {error}"),
        }
    }
}

// program associated functions
impl<'agent> ProgramManager<'agent> {
    /// Get a program from the network by its ID. This method will return an error if it does not exist.
    pub fn get_program_from_chain(program_id: &ProgramID) -> Result<Program> {
        let client = ureq::Agent::new();
        // Perform the request.
        let url = format!(
            "{}/{}/program/{}",
            DEFAULT_BASE_URL, DEFAULT_TESTNET, program_id.to_string()
        );
        match client.get(&url).call()?.into_json() {
            Ok(program) => Ok(program),
            Err(error) => bail!("Failed to parse program {program_id}: {error}"),
        }
    }

    /// Resolve imports of a program in a depth-first-search order from program source code
    ///
    /// # Arguments
    /// * `program` - The program to resolve imports for
    ///
    /// # Returns
    /// A map of program IDs to programs
    pub fn get_import_programs_from_chain(
        program: &Program,
    ) -> Result<IndexMap<ProgramID, Program>> {
        let mut found_imports = IndexMap::new();
        for (import_id, _) in program.imports().iter() {
            let imported_program = Self::get_program_from_chain(import_id)?;
            let nested_imports = Self::get_import_programs_from_chain(&imported_program)?;
            for (id, import) in nested_imports.into_iter() {
                found_imports
                    .contains_key(&id)
                    .then(|| anyhow!("Circular dependency discovered in program imports"));
                found_imports.insert(id, import);
            }
            found_imports
                .contains_key(import_id)
                .then(|| anyhow!("Circular dependency discovered in program imports"));
            found_imports.insert(*import_id, imported_program);
        }
        Ok(found_imports)
    }

    /// Load a program from a file path
    ///
    /// # Arguments
    /// * path - The path refers to the folder containing the program.json and *.aleo files,
    /// which are generated by `leo build` in the Leo project.
    pub fn load_program_from_path<P: Into<PathBuf>>(path: P) -> Result<Program> {
        let path = path.into();
        ensure!(path.exists(), "The program directory does not exist");
        let package = Package::open(&path)?;
        let program_name = package.program().id().name();
        ensure!(
            !Program::is_reserved_keyword(program_name),
            "Program name is invalid (reserved): {}",
            program_name
        );
        // Load the main program.
        Ok(package.program().clone())
    }

    /// Initialize a SnarkVM instance with a program and its imports
    fn initialize_vm(program: &Program) -> Result<VM> {
        // Create an ephemeral SnarkVM to store the programs
        // Initialize an RNG and query object for the transaction
        let store = ConsensusStore::open(None)?;
        let vm = VM::from(store)?;

        // Resolve imports
        let credits_id = ProgramID::from_str("credits.aleo")?;
        Self::get_import_programs_from_chain(program)?
            .iter()
            .try_for_each(|(_, import)| {
                if import.id() != &credits_id {
                    vm.process().write().add_program(import)?
                }
                Ok::<_, Error>(())
            })?;

        // If the initialization is for an execution, add the program. Otherwise, don't add it as
        // it will be added during the deployment process
        vm.process().write().add_program(program)?;
        Ok(vm)
    }
}