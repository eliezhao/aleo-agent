//! program deployment implementation

use std::str::FromStr;

use anyhow::{bail, ensure, Error};

use crate::agent::Agent;
use crate::program::ProgramManager;

use super::*;

impl Agent {
    /// Deploy a program to the network
    ///
    /// # Arguments
    ///  * `program` - The program to deploy
    ///  * `priority_fee` - The priority fee to pay for the deployment
    ///  * `fee_record` - The fee record to pay for deployment costs
    ///
    /// # Returns
    /// * The transaction hash of the deployment transaction
    pub fn deploy_program(
        &self,
        program: &Program,
        priority_fee: u64,
        fee_record: Option<PlaintextRecord>,
    ) -> anyhow::Result<String> {
        // Check if program is already deployed on chain, cancel deployment if so
        let program_id = program.id();
        ensure!(
            ProgramManager::get_program_from_chain(program_id).is_err(),
            "❌ Program {:?} already deployed on chain, cancelling deployment",
            program_id
        );

        // If the program has imports, check if they are deployed on chain. If not, cancel deployment
        program.imports().keys().try_for_each(|program_id| {
            if ProgramManager::get_program_from_chain(program_id).is_err() {
                bail!("❌ Imported program {program_id:?} could not be found on the Aleo Network, please deploy this imported program first before continuing with deployment of {program_id:?}");
            }
            Ok(())
        })?;

        let private_key = self.account().private_key();

        // Create the deployment transaction
        let transaction = self.create_deploy_transaction(
            program,
            private_key,
            self.base_url(),
            priority_fee,
            fee_record,
        )?;

        self.broadcast_transaction(&transaction)
    }

    /// Create a deployment transaction for a program without instantiating the program manager
    fn create_deploy_transaction(
        &self,
        program: &Program,
        private_key: &PrivateKey,
        node_url: &String,
        priority_fee: u64,
        fee_record: Option<PlaintextRecord>,
    ) -> anyhow::Result<Transaction> {
        // Initialize an RNG.
        let rng = &mut rand::thread_rng();
        let query = Query::from(node_url);

        // Initialize the VM
        let vm = Self::initialize_vm(program)?;

        // Create the deployment transaction
        vm.deploy(
            private_key,
            program,
            fee_record,
            priority_fee,
            Some(query),
            rng,
        )
    }

    fn initialize_vm(program: &Program) -> anyhow::Result<VM> {
        // Create an ephemeral SnarkVM to store the programs
        // Initialize an RNG and query object for the transaction
        let store = ConsensusStore::open(None)?;
        let vm = VM::from(store)?;

        // Resolve imports
        let credits_id = ProgramID::from_str("credits.aleo")?;
        ProgramManager::get_import_programs_from_chain(program)?
            .iter()
            .try_for_each(|(_, import)| {
                if import.id() != &credits_id {
                    vm.process().write().add_program(import)?
                }
                Ok::<_, Error>(())
            })?;
        Ok(vm)
    }
}
