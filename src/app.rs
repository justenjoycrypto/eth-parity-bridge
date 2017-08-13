use std::path::{Path, PathBuf};
use std::io;
use std::time::Duration;
use std::sync::Arc;
use futures::{future, Future};
use tokio_core::reactor::{Handle};
use web3::{Web3, Transport};
use web3::transports::ipc::Ipc;
use web3::types::TransactionRequest;
use error::{Error, ErrorKind, ResultExt};
use config::Config;
use database::Database;
use contracts::{EthereumBridge, KovanBridge};
use api;

pub enum Deployed {
	New(Database),
	None(Database),
}

pub struct App<T> where T: Transport {
	pub config: Config,
	pub database_path: PathBuf,
	pub connections: Connections<T>,
}

pub struct Connections<T> where T: Transport {
	pub mainnet: T,
	pub testnet: T,
}

impl Connections<Ipc> {
	pub fn new_ipc<P: AsRef<Path>>(handle: &Handle, mainnet: P, testnet: P) -> Result<Self, Error> {
		let mainnet = Ipc::with_event_loop(mainnet, handle).chain_err(|| "Cannot connect to mainnet node ipc")?;
		let testnet = Ipc::with_event_loop(testnet, handle).chain_err(|| "Cannot connect to testnet node ipc")?;

		let result = Connections {
			mainnet,
			testnet,
		};
		Ok(result)
	}
}

impl App<Ipc> {
	pub fn new_ipc<P: AsRef<Path>>(config: Config, database_path: P, handle: &Handle) -> Result<Self, Error> {
		let connections = Connections::new_ipc(handle, &config.mainnet.ipc, &config.testnet.ipc)?;
		let result = App {
			config,
			database_path: database_path.as_ref().to_path_buf(),
			connections,
		};
		Ok(result)
	}
}

impl<T: Transport> App<T> {
	pub fn ensure_deployed<'a>(&'a self) -> Box<Future<Item = Deployed, Error = Error> + 'a> {
		let database_path = self.database_path.clone();
		match Database::load(&database_path).map_err(ErrorKind::from) {
			Ok(database) => future::result(Ok(Deployed::None(database))).boxed(),
			Err(ErrorKind::MissingFile(_)) => Box::new(self.deploy().map(Deployed::New)),
			Err(err) => future::result(Err(err.into())).boxed(),
		}

	}

	pub fn deploy<'a>(&'a self) -> Box<Future<Item = Database, Error = Error> + 'a> {
		let main_tx_request = TransactionRequest {
			from: self.config.mainnet.account,
			to: None,
			gas: Some(self.config.mainnet.txs.deploy.gas.into()),
			gas_price: Some(self.config.mainnet.txs.deploy.gas_price.into()),
			value: Some(self.config.mainnet.txs.deploy.value.into()),
			data: Some(include_bytes!("../contracts/EthereumBridge.bin").to_vec().into()),
			nonce: None,
			condition: None,
		};

		let test_tx_request = TransactionRequest {
			from: self.config.testnet.account,
			to: None,
			gas: Some(self.config.testnet.txs.deploy.gas.into()),
			gas_price: Some(self.config.testnet.txs.deploy.gas_price.into()),
			value: Some(self.config.testnet.txs.deploy.value.into()),
			data: Some(include_bytes!("../contracts/KovanBridge.bin").to_vec().into()),
			nonce: None,
			condition: None,
		};

		let main_future = api::send_transaction_with_confirmation(&self.connections.mainnet, main_tx_request, self.config.mainnet.poll_interval, self.config.mainnet.required_confirmations);
		let test_future = api::send_transaction_with_confirmation(&self.connections.testnet, test_tx_request, self.config.testnet.poll_interval, self.config.testnet.required_confirmations);

		let deploy = main_future.join(test_future)
			.map(|(main_receipt, test_receipt)| {
				Database {
					mainnet_contract_address: main_receipt.contract_address.expect("contract creation receipt must have an address; qed"),
					testnet_contract_address: test_receipt.contract_address.expect("contract creation receipt must have an address; qed"),
					mainnet_deploy: main_receipt.block_number.low_u64(),
					testnet_deploy: test_receipt.block_number.low_u64(),
					checked_deposit_relay: main_receipt.block_number.low_u64(),
					checked_withdraw_relay: test_receipt.block_number.low_u64(),
					checked_withdraw_confirm: test_receipt.block_number.low_u64(),
				}
			})
			.map_err(ErrorKind::Web3)
			.map_err(Error::from)
			.map_err(|e| e.chain_err(|| "Failed to deploy contracts"));

		Box::new(deploy)
	}

	pub fn mainnet_bridge(&self) -> EthereumBridge {
		EthereumBridge(&self.config.mainnet.contract.abi)
	}

	pub fn testnet_bridge(&self) -> KovanBridge {
		KovanBridge(&self.config.mainnet.contract.abi)
	}
}


