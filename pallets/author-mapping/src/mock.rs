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

//! A minimal runtime including the author-mapping pallet
use crate as pallet_author_mapping;
use frame_support::{
	construct_runtime, parameter_types,
	traits::{Everything, GenesisBuild},
	weights::Weight,
};
use parity_scale_codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::H256;
use sp_io;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	Perbill, RuntimeDebug,
};

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Eq, PartialEq, Clone, Copy, Encode, Decode, RuntimeDebug)]
pub enum TestAuthor {
	Alice,
	Bob,
	Charlie,
	Dave,
	Eve,
}
impl Default for TestAuthor {
	fn default() -> TestAuthor {
		TestAuthor::Alice
	}
}
pub type AccountId = u64;
pub type Balance = u128;
pub type BlockNumber = u64;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		AuthorMapping: pallet_author_mapping::{Pallet, Call, Storage, Config<T>, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}
impl frame_system::Config for Test {
	type BaseCallFilter = Everything;
	type DbWeight = ();
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Call = Call;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type BlockWeights = ();
	type BlockLength = ();
	type SS58Prefix = ();
	type OnSetCode = ();
}
parameter_types! {
	pub const ExistentialDeposit: u128 = 1;
}
impl pallet_balances::Config for Test {
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 4];
	type MaxLocks = ();
	type Balance = Balance;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
}

parameter_types! {
	pub const DepositAmount: Balance = 100;
}
impl pallet_author_mapping::Config for Test {
	type Event = Event;
	type AuthorId = TestAuthor;
	type DepositCurrency = Balances;
	type DepositAmount = DepositAmount;
	type WeightInfo = ();
}

/// Externality builder for pallet author mapping's mock runtime
/// Allows configuring balances and initial mappings
pub(crate) struct ExtBuilder {
	/// Accounts endowed with balances
	balances: Vec<(AccountId, Balance)>,
	/// AuthorId -> AccoutId mappings
	mappings: Vec<(TestAuthor, AccountId)>,
}

impl Default for ExtBuilder {
	fn default() -> ExtBuilder {
		ExtBuilder {
			balances: vec![],
			mappings: vec![],
		}
	}
}

impl ExtBuilder {
	pub(crate) fn with_balances(mut self, balances: Vec<(AccountId, Balance)>) -> Self {
		self.balances = balances;
		self
	}

	pub(crate) fn with_mappings(mut self, mappings: Vec<(TestAuthor, AccountId)>) -> Self {
		self.mappings = mappings;
		self
	}

	pub(crate) fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Test>()
			.expect("Frame system builds valid default genesis config");

		pallet_balances::GenesisConfig::<Test> {
			balances: self.balances,
		}
		.assimilate_storage(&mut t)
		.expect("Pallet balances storage can be assimilated");

		pallet_author_mapping::GenesisConfig::<Test> {
			mappings: self.mappings,
		}
		.assimilate_storage(&mut t)
		.expect("Pallet author mapping's storage can be assimilated");

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

pub(crate) fn last_event() -> Event {
	System::events().pop().expect("Event expected").event
}
