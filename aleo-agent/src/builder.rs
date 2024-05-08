//! A builder for an [Agent]

use crate::account::Account;
use crate::agent::Agent;
use crate::{DEFAULT_BASE_URL, DEFAULT_TESTNET};

#[derive(Clone)]
pub struct AgentBuilder {
    url: String,
    network: String,
    account: Account,
}

impl Default for AgentBuilder {
    fn default() -> Self {
        AgentBuilder {
            url: DEFAULT_BASE_URL.to_string(),
            network: DEFAULT_TESTNET.to_string(),
            account: Account::default(),
        }
    }
}

impl AgentBuilder {
    pub fn build(self) -> Agent {
        Agent::new(self.url, self.network, self.account)
    }

    pub fn with_url<S: Into<String>>(mut self, url: S) -> Self {
        self.url = url.into();
        self
    }

    pub fn with_network<S: Into<String>>(mut self, network: S) -> Self {
        self.network = network.into();
        self
    }

    pub fn with_account(mut self, account: Account) -> Self {
        self.account = account;
        self
    }
}
