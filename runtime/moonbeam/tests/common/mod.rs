// Copyright 2019-2021 PureStake Inc.
// This file is part of Moonbeam.

// Moonbeam is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Moonbeam is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Moonbeam.  If not, see <http://www.gnu.org/licenses/>.

#![allow(dead_code)]

use cumulus_primitives_parachain_inherent::ParachainInherentData;
use frame_support::{
	assert_ok,
	dispatch::Dispatchable,
	traits::{GenesisBuild, OnFinalize, OnInitialize},
};
pub use moonbeam_runtime::{
	currency::GLMR, AccountId, AuthorInherent, Balance, Balances, Call, CrowdloanRewards, Ethereum,
	Event, Executive, FixedGasPrice, InflationInfo, ParachainStaking, Range, Runtime, System,
	TransactionConverter, UncheckedExtrinsic, WEEKS,
};
use nimbus_primitives::NimbusId;
use pallet_evm::GenesisAccount;
use sp_core::H160;
use sp_runtime::Perbill;

use std::collections::BTreeMap;

use fp_rpc::ConvertTransaction;

// {from: 0x6be02d1d3665660d22ff9624b7be0551ee1ac91b, .., gasPrice: "0x01"}
pub const VALID_ETH_TX: &str =
	"f86880843b9aca0083b71b0094111111111111111111111111111111111111111182020080820a26a\
	08c69faf613b9f72dbb029bb5d5acf42742d214c79743507e75fdc8adecdee928a001be4f58ff278ac\
	61125a81a582a717d9c5d6554326c01b878297c6522b12282";

// Gas limit < transaction cost.
pub const INVALID_ETH_TX: &str =
	"f86180843b9aca00809412cb274aad8251c875c0bf6872b67d9983e53fdd01801ca00e28ba2dd3c5a\
	3fd467d4afd7aefb4a34b373314fff470bb9db743a84d674a0aa06e5994f2d07eafe1c37b4ce5471ca\
	ecec29011f6f5bf0b1a552c55ea348df35f";

pub fn run_to_block(n: u32) {
	while System::block_number() < n {
		Ethereum::on_finalize(System::block_number());
		AuthorInherent::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		AuthorInherent::on_initialize(System::block_number());
		ParachainStaking::on_initialize(System::block_number());
		Ethereum::on_initialize(System::block_number());
	}
}

pub fn last_event() -> Event {
	System::events().pop().expect("Event expected").event
}

// Helper function to give a simple evm context suitable for tests.
// We can remove this once https://github.com/rust-blockchain/evm/pull/35
// is in our dependency graph.
pub fn evm_test_context() -> evm::Context {
	evm::Context {
		address: Default::default(),
		caller: Default::default(),
		apparent_value: From::from(0),
	}
}

pub struct ExtBuilder {
	// endowed accounts with balances
	balances: Vec<(AccountId, Balance)>,
	// [collator, amount]
	collators: Vec<(AccountId, Balance)>,
	// [nominator, collator, nomination_amount]
	nominations: Vec<(AccountId, AccountId, Balance)>,
	// per-round inflation config
	inflation: InflationInfo<Balance>,
	// AuthorId -> AccoutId mappings
	mappings: Vec<(NimbusId, AccountId)>,
	// Crowdloan fund
	crowdloan_fund: Balance,
	// Chain id
	chain_id: u64,
	// EVM genesis accounts
	evm_accounts: BTreeMap<H160, GenesisAccount>,
}

impl Default for ExtBuilder {
	fn default() -> ExtBuilder {
		ExtBuilder {
			balances: vec![],
			nominations: vec![],
			collators: vec![],
			inflation: InflationInfo {
				expect: Range {
					min: 100_000 * GLMR,
					ideal: 200_000 * GLMR,
					max: 500_000 * GLMR,
				},
				// not used
				annual: Range {
					min: Perbill::from_percent(50),
					ideal: Perbill::from_percent(50),
					max: Perbill::from_percent(50),
				},
				// unrealistically high parameterization, only for testing
				round: Range {
					min: Perbill::from_percent(5),
					ideal: Perbill::from_percent(5),
					max: Perbill::from_percent(5),
				},
			},
			mappings: vec![],
			crowdloan_fund: 0,
			chain_id: CHAIN_ID,
			evm_accounts: BTreeMap::new(),
		}
	}
}

impl ExtBuilder {
	pub fn with_evm_accounts(mut self, accounts: BTreeMap<H160, GenesisAccount>) -> Self {
		self.evm_accounts = accounts;
		self
	}

	pub fn with_balances(mut self, balances: Vec<(AccountId, Balance)>) -> Self {
		self.balances = balances;
		self
	}

	pub fn with_collators(mut self, collators: Vec<(AccountId, Balance)>) -> Self {
		self.collators = collators;
		self
	}

	pub fn with_nominations(mut self, nominations: Vec<(AccountId, AccountId, Balance)>) -> Self {
		self.nominations = nominations;
		self
	}

	pub fn with_crowdloan_fund(mut self, crowdloan_fund: Balance) -> Self {
		self.crowdloan_fund = crowdloan_fund;
		self
	}

	pub fn with_mappings(mut self, mappings: Vec<(NimbusId, AccountId)>) -> Self {
		self.mappings = mappings;
		self
	}

	#[allow(dead_code)]
	pub fn with_inflation(mut self, inflation: InflationInfo<Balance>) -> Self {
		self.inflation = inflation;
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		pallet_balances::GenesisConfig::<Runtime> {
			balances: self.balances,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		parachain_staking::GenesisConfig::<Runtime> {
			candidates: self.collators,
			nominations: self.nominations,
			inflation_config: self.inflation,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		pallet_crowdloan_rewards::GenesisConfig::<Runtime> {
			funded_amount: self.crowdloan_fund,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		pallet_author_mapping::GenesisConfig::<Runtime> {
			mappings: self.mappings,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		<pallet_ethereum_chain_id::GenesisConfig as GenesisBuild<Runtime>>::assimilate_storage(
			&pallet_ethereum_chain_id::GenesisConfig {
				chain_id: self.chain_id,
			},
			&mut t,
		)
		.unwrap();

		<pallet_evm::GenesisConfig as GenesisBuild<Runtime>>::assimilate_storage(
			&pallet_evm::GenesisConfig {
				accounts: self.evm_accounts,
			},
			&mut t,
		)
		.unwrap();

		<pallet_ethereum::GenesisConfig as GenesisBuild<Runtime>>::assimilate_storage(
			&pallet_ethereum::GenesisConfig {},
			&mut t,
		)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

pub const CHAIN_ID: u64 = 1281;
pub const ALICE: [u8; 20] = [4u8; 20];
pub const ALICE_NIMBUS: [u8; 32] = [4u8; 32];
pub const BOB: [u8; 20] = [5u8; 20];
pub const CHARLIE: [u8; 20] = [6u8; 20];
pub const DAVE: [u8; 20] = [7u8; 20];
pub const EVM_CONTRACT: [u8; 20] = [8u8; 20];

pub fn origin_of(account_id: AccountId) -> <Runtime as frame_system::Config>::Origin {
	<Runtime as frame_system::Config>::Origin::signed(account_id)
}

pub fn inherent_origin() -> <Runtime as frame_system::Config>::Origin {
	<Runtime as frame_system::Config>::Origin::none()
}

pub fn root_origin() -> <Runtime as frame_system::Config>::Origin {
	<Runtime as frame_system::Config>::Origin::root()
}

/// Mock the inherent that sets author in `author-inherent`
pub fn set_author(a: NimbusId) {
	assert_ok!(
		Call::AuthorInherent(pallet_author_inherent::Call::<Runtime>::set_author(a))
			.dispatch(inherent_origin())
	);
}

/// Mock the inherent that sets validation data in ParachainSystem, which
/// contains the `relay_chain_block_number`, which is used in `author-filter` as a
/// source of randomness to filter valid authors at each block.
pub fn set_parachain_inherent_data() {
	use cumulus_primitives_core::PersistedValidationData;
	use cumulus_test_relay_sproof_builder::RelayStateSproofBuilder;
	let (relay_parent_storage_root, relay_chain_state) =
		RelayStateSproofBuilder::default().into_state_root_and_proof();
	let vfp = PersistedValidationData {
		relay_parent_number: 1u32,
		relay_parent_storage_root,
		..Default::default()
	};
	let parachain_inherent_data = ParachainInherentData {
		validation_data: vfp,
		relay_chain_state: relay_chain_state,
		downward_messages: Default::default(),
		horizontal_messages: Default::default(),
	};
	assert_ok!(Call::ParachainSystem(
		cumulus_pallet_parachain_system::Call::<Runtime>::set_validation_data(
			parachain_inherent_data
		)
	)
	.dispatch(inherent_origin()));
}

pub fn unchecked_eth_tx(raw_hex_tx: &str) -> UncheckedExtrinsic {
	let converter = TransactionConverter;
	converter.convert_transaction(ethereum_transaction(raw_hex_tx))
}

pub fn ethereum_transaction(raw_hex_tx: &str) -> pallet_ethereum::Transaction {
	let bytes = hex::decode(raw_hex_tx).expect("Transaction bytes.");
	let transaction = rlp::decode::<pallet_ethereum::Transaction>(&bytes[..]);
	assert!(transaction.is_ok());
	transaction.unwrap()
}
