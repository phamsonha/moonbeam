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

//! Moonriver Runtime Integration Tests

#![cfg(test)]

mod common;
use common::*;

use evm::{executor::PrecompileOutput, Context, ExitError, ExitSucceed};
use frame_support::{
	assert_noop, assert_ok,
	dispatch::Dispatchable,
	traits::{fungible::Inspect, PalletInfo, StorageInfo, StorageInfoTrait},
	weights::{DispatchClass, Weight},
	StorageHasher, Twox128,
};
use moonriver_runtime::{BlockWeights, Precompiles};
use nimbus_primitives::NimbusId;
use pallet_evm::PrecompileSet;
use pallet_transaction_payment::Multiplier;
use parachain_staking::{Bond, NominatorAdded};
use sha3::{Digest, Keccak256};
use sp_core::{Public, H160, U256};
use sp_runtime::{
	traits::{Convert, One},
	DispatchError,
};

#[test]
fn fast_track_available() {
	assert!(<moonriver_runtime::Runtime as pallet_democracy::Config>::InstantAllowed::get());
}

#[test]
fn verify_pallet_prefixes() {
	fn is_pallet_prefix<P: 'static>(name: &str) {
		// Compares the unhashed pallet prefix in the `StorageInstance` implementation by every
		// storage item in the pallet P. This pallet prefix is used in conjunction with the
		// item name to get the unique storage key: hash(PalletPrefix) + hash(StorageName)
		// https://github.com/paritytech/substrate/blob/master/frame/support/procedural/src/pallet/
		// expand/storage.rs#L389-L401
		assert_eq!(
			<moonriver_runtime::Runtime as frame_system::Config>::PalletInfo::name::<P>(),
			Some(name)
		);
	}
	// TODO: use StorageInfoTrait from https://github.com/paritytech/substrate/pull/9246
	// This is now available with polkadot-v0.9.9 dependencies
	is_pallet_prefix::<moonriver_runtime::System>("System");
	is_pallet_prefix::<moonriver_runtime::Utility>("Utility");
	is_pallet_prefix::<moonriver_runtime::RandomnessCollectiveFlip>("RandomnessCollectiveFlip");
	is_pallet_prefix::<moonriver_runtime::ParachainSystem>("ParachainSystem");
	is_pallet_prefix::<moonriver_runtime::TransactionPayment>("TransactionPayment");
	is_pallet_prefix::<moonriver_runtime::ParachainInfo>("ParachainInfo");
	is_pallet_prefix::<moonriver_runtime::EthereumChainId>("EthereumChainId");
	is_pallet_prefix::<moonriver_runtime::EVM>("EVM");
	is_pallet_prefix::<moonriver_runtime::Ethereum>("Ethereum");
	is_pallet_prefix::<moonriver_runtime::ParachainStaking>("ParachainStaking");
	is_pallet_prefix::<moonriver_runtime::MaintenanceMode>("MaintenanceMode");
	is_pallet_prefix::<moonriver_runtime::Scheduler>("Scheduler");
	is_pallet_prefix::<moonriver_runtime::Democracy>("Democracy");
	is_pallet_prefix::<moonriver_runtime::CouncilCollective>("CouncilCollective");
	is_pallet_prefix::<moonriver_runtime::TechComitteeCollective>("TechComitteeCollective");
	is_pallet_prefix::<moonriver_runtime::Treasury>("Treasury");
	is_pallet_prefix::<moonriver_runtime::AuthorInherent>("AuthorInherent");
	is_pallet_prefix::<moonriver_runtime::AuthorFilter>("AuthorFilter");
	is_pallet_prefix::<moonriver_runtime::CrowdloanRewards>("CrowdloanRewards");
	is_pallet_prefix::<moonriver_runtime::AuthorMapping>("AuthorMapping");
	let prefix = |pallet_name, storage_name| {
		let mut res = [0u8; 32];
		res[0..16].copy_from_slice(&Twox128::hash(pallet_name));
		res[16..32].copy_from_slice(&Twox128::hash(storage_name));
		res.to_vec()
	};
	assert_eq!(
		<moonriver_runtime::Timestamp as StorageInfoTrait>::storage_info(),
		vec![
			StorageInfo {
				pallet_name: b"Timestamp".to_vec(),
				storage_name: b"Now".to_vec(),
				prefix: prefix(b"Timestamp", b"Now"),
				max_values: Some(1),
				max_size: Some(8),
			},
			StorageInfo {
				pallet_name: b"Timestamp".to_vec(),
				storage_name: b"DidUpdate".to_vec(),
				prefix: prefix(b"Timestamp", b"DidUpdate"),
				max_values: Some(1),
				max_size: Some(1),
			}
		]
	);
	assert_eq!(
		<moonriver_runtime::Balances as StorageInfoTrait>::storage_info(),
		vec![
			StorageInfo {
				pallet_name: b"Balances".to_vec(),
				storage_name: b"TotalIssuance".to_vec(),
				prefix: prefix(b"Balances", b"TotalIssuance"),
				max_values: Some(1),
				max_size: Some(16),
			},
			StorageInfo {
				pallet_name: b"Balances".to_vec(),
				storage_name: b"Account".to_vec(),
				prefix: prefix(b"Balances", b"Account"),
				max_values: Some(300_000),
				max_size: Some(100),
			},
			StorageInfo {
				pallet_name: b"Balances".to_vec(),
				storage_name: b"Locks".to_vec(),
				prefix: prefix(b"Balances", b"Locks"),
				max_values: Some(300_000),
				max_size: Some(1287),
			},
			StorageInfo {
				pallet_name: b"Balances".to_vec(),
				storage_name: b"Reserves".to_vec(),
				prefix: prefix(b"Balances", b"Reserves"),
				max_values: None,
				max_size: Some(1037),
			},
			StorageInfo {
				pallet_name: b"Balances".to_vec(),
				storage_name: b"StorageVersion".to_vec(),
				prefix: prefix(b"Balances", b"StorageVersion"),
				max_values: Some(1),
				max_size: Some(1),
			}
		]
	);
	assert_eq!(
		<moonriver_runtime::Proxy as StorageInfoTrait>::storage_info(),
		vec![
			StorageInfo {
				pallet_name: b"Proxy".to_vec(),
				storage_name: b"Proxies".to_vec(),
				prefix: prefix(b"Proxy", b"Proxies"),
				max_values: None,
				max_size: Some(845),
			},
			StorageInfo {
				pallet_name: b"Proxy".to_vec(),
				storage_name: b"Announcements".to_vec(),
				prefix: prefix(b"Proxy", b"Announcements"),
				max_values: None,
				max_size: Some(1837),
			}
		]
	);
	assert_eq!(
		<moonriver_runtime::MaintenanceMode as StorageInfoTrait>::storage_info(),
		vec![StorageInfo {
			pallet_name: b"MaintenanceMode".to_vec(),
			storage_name: b"MaintenanceMode".to_vec(),
			prefix: prefix(b"MaintenanceMode", b"MaintenanceMode"),
			max_values: Some(1),
			max_size: Some(1),
		},]
	);
}

#[test]
fn verify_pallet_indices() {
	fn is_pallet_index<P: 'static>(index: usize) {
		assert_eq!(
			<moonriver_runtime::Runtime as frame_system::Config>::PalletInfo::index::<P>(),
			Some(index)
		);
	}
	// System support
	is_pallet_index::<moonriver_runtime::System>(0);
	is_pallet_index::<moonriver_runtime::ParachainSystem>(1);
	is_pallet_index::<moonriver_runtime::RandomnessCollectiveFlip>(2);
	is_pallet_index::<moonriver_runtime::Timestamp>(3);
	is_pallet_index::<moonriver_runtime::ParachainInfo>(4);
	// Monetary
	is_pallet_index::<moonriver_runtime::Balances>(10);
	is_pallet_index::<moonriver_runtime::TransactionPayment>(11);
	// Consensus support
	is_pallet_index::<moonriver_runtime::ParachainStaking>(20);
	is_pallet_index::<moonriver_runtime::AuthorInherent>(21);
	is_pallet_index::<moonriver_runtime::AuthorFilter>(22);
	is_pallet_index::<moonriver_runtime::AuthorMapping>(23);
	// Handy utilities
	is_pallet_index::<moonriver_runtime::Utility>(30);
	is_pallet_index::<moonriver_runtime::Proxy>(31);
	is_pallet_index::<moonriver_runtime::MaintenanceMode>(32);
	// TODO Sudo was previously index 40, should we test that there is nothing there now?
	// Ethereum compatibility
	is_pallet_index::<moonriver_runtime::EthereumChainId>(50);
	is_pallet_index::<moonriver_runtime::EVM>(51);
	is_pallet_index::<moonriver_runtime::Ethereum>(52);
	// Governance
	is_pallet_index::<moonriver_runtime::Scheduler>(60);
	is_pallet_index::<moonriver_runtime::Democracy>(61);
	// Council
	is_pallet_index::<moonriver_runtime::CouncilCollective>(70);
	is_pallet_index::<moonriver_runtime::TechComitteeCollective>(71);
	// Treasury
	is_pallet_index::<moonriver_runtime::Treasury>(80);
	// Crowdloan
	is_pallet_index::<moonriver_runtime::CrowdloanRewards>(90);
}

#[test]
fn join_collator_candidates() {
	ExtBuilder::default()
		.with_balances(vec![
			(AccountId::from(ALICE), 2_000 * MOVR),
			(AccountId::from(BOB), 2_000 * MOVR),
			(AccountId::from(CHARLIE), 1_100 * MOVR),
			(AccountId::from(DAVE), 1_000 * MOVR),
		])
		.with_collators(vec![
			(AccountId::from(ALICE), 1_000 * MOVR),
			(AccountId::from(BOB), 1_000 * MOVR),
		])
		.with_nominations(vec![
			(AccountId::from(CHARLIE), AccountId::from(ALICE), 50 * MOVR),
			(AccountId::from(CHARLIE), AccountId::from(BOB), 50 * MOVR),
		])
		.build()
		.execute_with(|| {
			assert_noop!(
				ParachainStaking::join_candidates(
					origin_of(AccountId::from(ALICE)),
					1_000 * MOVR,
					2u32
				),
				parachain_staking::Error::<Runtime>::CandidateExists
			);
			assert_noop!(
				ParachainStaking::join_candidates(
					origin_of(AccountId::from(CHARLIE)),
					1_000 * MOVR,
					2u32
				),
				parachain_staking::Error::<Runtime>::NominatorExists
			);
			assert!(System::events().is_empty());
			assert_ok!(ParachainStaking::join_candidates(
				origin_of(AccountId::from(DAVE)),
				1_000 * MOVR,
				2u32
			));
			assert_eq!(
				last_event(),
				Event::ParachainStaking(parachain_staking::Event::JoinedCollatorCandidates(
					AccountId::from(DAVE),
					1_000 * MOVR,
					3_100 * MOVR
				))
			);
			let candidates = ParachainStaking::candidate_pool();
			assert_eq!(
				candidates.0[0],
				Bond {
					owner: AccountId::from(ALICE),
					amount: 1_050 * MOVR
				}
			);
			assert_eq!(
				candidates.0[1],
				Bond {
					owner: AccountId::from(BOB),
					amount: 1_050 * MOVR
				}
			);
			assert_eq!(
				candidates.0[2],
				Bond {
					owner: AccountId::from(DAVE),
					amount: 1_000 * MOVR
				}
			);
		});
}

#[test]
fn transfer_through_evm_to_stake() {
	ExtBuilder::default()
		.with_balances(vec![(AccountId::from(ALICE), 2_000 * MOVR)])
		.build()
		.execute_with(|| {
			// Charlie has no balance => fails to stake
			assert_noop!(
				ParachainStaking::join_candidates(
					origin_of(AccountId::from(CHARLIE)),
					1_000 * MOVR,
					2u32
				),
				DispatchError::Module {
					index: 10,
					error: 2,
					message: Some("InsufficientBalance")
				}
			);
			// Alice transfer from free balance 2000 MOVR to Bob
			assert_ok!(Balances::transfer(
				origin_of(AccountId::from(ALICE)),
				AccountId::from(BOB),
				2_000 * MOVR,
			));
			assert_eq!(Balances::free_balance(AccountId::from(BOB)), 2_000 * MOVR);

			let gas_limit = 100000u64;
			let gas_price: U256 = 1_000_000_000.into();
			// Bob transfers 1000 MOVR to Charlie via EVM
			assert_ok!(Call::EVM(pallet_evm::Call::<Runtime>::call(
				AccountId::from(BOB),
				AccountId::from(CHARLIE),
				Vec::new(),
				(1_000 * MOVR).into(),
				gas_limit,
				gas_price,
				None
			))
			.dispatch(<Runtime as frame_system::Config>::Origin::root()));
			assert_eq!(
				Balances::free_balance(AccountId::from(CHARLIE)),
				1_000 * MOVR,
			);

			// Charlie can stake now
			assert_ok!(ParachainStaking::join_candidates(
				origin_of(AccountId::from(CHARLIE)),
				1_000 * MOVR,
				2u32,
			),);
			let candidates = ParachainStaking::candidate_pool();
			assert_eq!(
				candidates.0[0],
				Bond {
					owner: AccountId::from(CHARLIE),
					amount: 1_000 * MOVR
				}
			);
		});
}

#[test]
fn reward_block_authors() {
	ExtBuilder::default()
		.with_balances(vec![
			// Alice gets 100 extra tokens for her mapping deposit
			(AccountId::from(ALICE), 2_100 * MOVR),
			(AccountId::from(BOB), 1_000 * MOVR),
		])
		.with_collators(vec![(AccountId::from(ALICE), 1_000 * MOVR)])
		.with_nominations(vec![(
			AccountId::from(BOB),
			AccountId::from(ALICE),
			500 * MOVR,
		)])
		.with_mappings(vec![(
			NimbusId::from_slice(&ALICE_NIMBUS),
			AccountId::from(ALICE),
		)])
		.build()
		.execute_with(|| {
			set_parachain_inherent_data();
			for x in 2..599 {
				set_author(NimbusId::from_slice(&ALICE_NIMBUS));
				run_to_block(x);
			}
			// no rewards doled out yet
			assert_eq!(Balances::free_balance(AccountId::from(ALICE)), 1_000 * MOVR,);
			assert_eq!(Balances::free_balance(AccountId::from(BOB)), 500 * MOVR,);
			set_author(NimbusId::from_slice(&ALICE_NIMBUS));
			run_to_block(600);
			// rewards minted and distributed
			assert_eq!(
				Balances::free_balance(AccountId::from(ALICE)),
				1113666666584000000000,
			);
			assert_eq!(
				Balances::free_balance(AccountId::from(BOB)),
				541333333292000000000,
			);
		});
}

#[test]
fn reward_block_authors_with_parachain_bond_reserved() {
	ExtBuilder::default()
		.with_balances(vec![
			// Alice gets 100 extra tokens for her mapping deposit
			(AccountId::from(ALICE), 2_100 * MOVR),
			(AccountId::from(BOB), 1_000 * MOVR),
			(AccountId::from(CHARLIE), MOVR),
		])
		.with_collators(vec![(AccountId::from(ALICE), 1_000 * MOVR)])
		.with_nominations(vec![(
			AccountId::from(BOB),
			AccountId::from(ALICE),
			500 * MOVR,
		)])
		.with_mappings(vec![(
			NimbusId::from_slice(&ALICE_NIMBUS),
			AccountId::from(ALICE),
		)])
		.build()
		.execute_with(|| {
			set_parachain_inherent_data();
			assert_ok!(ParachainStaking::set_parachain_bond_account(
				root_origin(),
				AccountId::from(CHARLIE),
			),);
			for x in 2..599 {
				set_author(NimbusId::from_slice(&ALICE_NIMBUS));
				run_to_block(x);
			}
			// no rewards doled out yet
			assert_eq!(Balances::free_balance(AccountId::from(ALICE)), 1_000 * MOVR,);
			assert_eq!(Balances::free_balance(AccountId::from(BOB)), 500 * MOVR,);
			assert_eq!(Balances::free_balance(AccountId::from(CHARLIE)), MOVR,);
			set_author(NimbusId::from_slice(&ALICE_NIMBUS));
			run_to_block(600);
			// rewards minted and distributed
			assert_eq!(
				Balances::free_balance(AccountId::from(ALICE)),
				1079592333275448000000,
			);
			assert_eq!(
				Balances::free_balance(AccountId::from(BOB)),
				528942666637724000000,
			);
			// 30% reserved for parachain bond
			assert_eq!(
				Balances::free_balance(AccountId::from(CHARLIE)),
				47515000000000000000,
			);
		});
}

#[test]
fn initialize_crowdloan_addresses_with_batch_and_pay() {
	ExtBuilder::default()
		.with_balances(vec![
			(AccountId::from(ALICE), 2_000 * MOVR),
			(AccountId::from(BOB), 1_000 * MOVR),
		])
		.with_collators(vec![(AccountId::from(ALICE), 1_000 * MOVR)])
		.with_mappings(vec![(
			NimbusId::from_slice(&ALICE_NIMBUS),
			AccountId::from(ALICE),
		)])
		.with_crowdloan_fund(3_000_000 * MOVR)
		.build()
		.execute_with(|| {
			// set parachain inherent data
			set_parachain_inherent_data();
			set_author(NimbusId::from_slice(&ALICE_NIMBUS));
			for x in 1..3 {
				run_to_block(x);
			}
			let init_block = CrowdloanRewards::init_relay_block();
			// This matches the previous vesting
			let end_block = init_block + 48 * WEEKS;
			// Batch calls always succeed. We just need to check the inner event
			assert_ok!(
				Call::Utility(pallet_utility::Call::<Runtime>::batch_all(vec![
					Call::CrowdloanRewards(
						pallet_crowdloan_rewards::Call::<Runtime>::initialize_reward_vec(vec![(
							[4u8; 32].into(),
							Some(AccountId::from(CHARLIE)),
							1_500_000 * MOVR
						)])
					),
					Call::CrowdloanRewards(
						pallet_crowdloan_rewards::Call::<Runtime>::initialize_reward_vec(vec![(
							[5u8; 32].into(),
							Some(AccountId::from(DAVE)),
							1_500_000 * MOVR
						)])
					),
					Call::CrowdloanRewards(
						pallet_crowdloan_rewards::Call::<Runtime>::complete_initialization(
							end_block
						)
					)
				]))
				.dispatch(root_origin())
			);
			// 30 percent initial payout
			assert_eq!(Balances::balance(&AccountId::from(CHARLIE)), 450_000 * MOVR);
			// 30 percent initial payout
			assert_eq!(Balances::balance(&AccountId::from(DAVE)), 450_000 * MOVR);
			let expected = Event::Utility(pallet_utility::Event::BatchCompleted);
			assert_eq!(last_event(), expected);
			// This one should fail, as we already filled our data
			assert_ok!(Call::Utility(pallet_utility::Call::<Runtime>::batch(vec![
				Call::CrowdloanRewards(
					pallet_crowdloan_rewards::Call::<Runtime>::initialize_reward_vec(vec![(
						[4u8; 32].into(),
						Some(AccountId::from(ALICE)),
						432000
					)])
				)
			]))
			.dispatch(root_origin()));
			let expected_fail = Event::Utility(pallet_utility::Event::BatchInterrupted(
				0,
				DispatchError::Module {
					index: 90,
					error: 8,
					message: None,
				},
			));
			assert_eq!(last_event(), expected_fail);
			// Claim 1 block.
			assert_ok!(CrowdloanRewards::claim(origin_of(AccountId::from(CHARLIE))));
			assert_ok!(CrowdloanRewards::claim(origin_of(AccountId::from(DAVE))));

			let vesting_period = 48 * WEEKS as u128;
			let per_block = (1_050_000 * MOVR) / vesting_period;

			assert_eq!(
				CrowdloanRewards::accounts_payable(&AccountId::from(CHARLIE))
					.unwrap()
					.claimed_reward,
				(450_000 * MOVR) + per_block
			);
			assert_eq!(
				CrowdloanRewards::accounts_payable(&AccountId::from(DAVE))
					.unwrap()
					.claimed_reward,
				(450_000 * MOVR) + per_block
			);
			// The total claimed reward should be equal to the account balance at this point.
			assert_eq!(
				Balances::balance(&AccountId::from(CHARLIE)),
				(450_000 * MOVR) + per_block
			);
			assert_eq!(
				Balances::balance(&AccountId::from(DAVE)),
				(450_000 * MOVR) + per_block
			);
			assert_noop!(
				CrowdloanRewards::claim(origin_of(AccountId::from(ALICE))),
				pallet_crowdloan_rewards::Error::<Runtime>::NoAssociatedClaim
			);
		});
}

#[test]
fn claim_via_precompile() {
	ExtBuilder::default()
		.with_balances(vec![
			(AccountId::from(ALICE), 2_000 * MOVR),
			(AccountId::from(BOB), 1_000 * MOVR),
		])
		.with_collators(vec![(AccountId::from(ALICE), 1_000 * MOVR)])
		.with_mappings(vec![(
			NimbusId::from_slice(&ALICE_NIMBUS),
			AccountId::from(ALICE),
		)])
		.with_crowdloan_fund(3_000_000 * MOVR)
		.build()
		.execute_with(|| {
			// set parachain inherent data
			set_parachain_inherent_data();
			set_author(NimbusId::from_slice(&ALICE_NIMBUS));
			for x in 1..3 {
				run_to_block(x);
			}
			let init_block = CrowdloanRewards::init_relay_block();
			// This matches the previous vesting
			let end_block = init_block + 4 * WEEKS;
			// Batch calls always succeed. We just need to check the inner event
			assert_ok!(
				Call::Utility(pallet_utility::Call::<Runtime>::batch_all(vec![
					Call::CrowdloanRewards(
						pallet_crowdloan_rewards::Call::<Runtime>::initialize_reward_vec(vec![(
							[4u8; 32].into(),
							Some(AccountId::from(CHARLIE)),
							1_500_000 * MOVR
						)])
					),
					Call::CrowdloanRewards(
						pallet_crowdloan_rewards::Call::<Runtime>::initialize_reward_vec(vec![(
							[5u8; 32].into(),
							Some(AccountId::from(DAVE)),
							1_500_000 * MOVR
						)])
					),
					Call::CrowdloanRewards(
						pallet_crowdloan_rewards::Call::<Runtime>::complete_initialization(
							end_block
						)
					)
				]))
				.dispatch(root_origin())
			);

			assert!(CrowdloanRewards::initialized());

			run_to_block(4);
			// 30 percent initial payout
			assert_eq!(Balances::balance(&AccountId::from(CHARLIE)), 450_000 * MOVR);
			// 30 percent initial payout
			assert_eq!(Balances::balance(&AccountId::from(DAVE)), 450_000 * MOVR);

			let crowdloan_precompile_address = H160::from_low_u64_be(2049);

			// Alice uses the staking precompile to go offline
			let gas_limit = 100000u64;
			let gas_price: U256 = 1_000_000_000.into();

			// Construct the call data (selector, amount)
			let mut call_data = Vec::<u8>::from([0u8; 4]);
			call_data[0..4].copy_from_slice(&Keccak256::digest(b"claim()")[0..4]);

			assert_ok!(Call::EVM(pallet_evm::Call::<Runtime>::call(
				AccountId::from(CHARLIE),
				crowdloan_precompile_address,
				call_data,
				U256::zero(), // No value sent in EVM
				gas_limit,
				gas_price,
				None, // Use the next nonce
			))
			.dispatch(<Runtime as frame_system::Config>::Origin::root()));

			let vesting_period = 4 * WEEKS as u128;
			let per_block = (1_050_000 * MOVR) / vesting_period;

			assert_eq!(
				CrowdloanRewards::accounts_payable(&AccountId::from(CHARLIE))
					.unwrap()
					.claimed_reward,
				(450_000 * MOVR) + per_block
			);
		})
}

#[test]
fn is_contributor_via_precompile() {
	ExtBuilder::default()
		.with_balances(vec![
			(AccountId::from(ALICE), 2_000 * MOVR),
			(AccountId::from(BOB), 1_000 * MOVR),
		])
		.with_collators(vec![(AccountId::from(ALICE), 1_000 * MOVR)])
		.with_mappings(vec![(
			NimbusId::from_slice(&ALICE_NIMBUS),
			AccountId::from(ALICE),
		)])
		.with_crowdloan_fund(3_000_000 * MOVR)
		.build()
		.execute_with(|| {
			// set parachain inherent data
			set_parachain_inherent_data();
			set_author(NimbusId::from_slice(&ALICE_NIMBUS));
			for x in 1..3 {
				run_to_block(x);
			}
			let init_block = CrowdloanRewards::init_relay_block();
			// This matches the previous vesting
			let end_block = init_block + 4 * WEEKS;
			// Batch calls always succeed. We just need to check the inner event
			assert_ok!(
				Call::Utility(pallet_utility::Call::<Runtime>::batch_all(vec![
					Call::CrowdloanRewards(
						pallet_crowdloan_rewards::Call::<Runtime>::initialize_reward_vec(vec![(
							[4u8; 32].into(),
							Some(AccountId::from(CHARLIE)),
							1_500_000 * MOVR
						)])
					),
					Call::CrowdloanRewards(
						pallet_crowdloan_rewards::Call::<Runtime>::initialize_reward_vec(vec![(
							[5u8; 32].into(),
							Some(AccountId::from(DAVE)),
							1_500_000 * MOVR
						)])
					),
					Call::CrowdloanRewards(
						pallet_crowdloan_rewards::Call::<Runtime>::complete_initialization(
							end_block
						)
					)
				]))
				.dispatch(root_origin())
			);

			let crowdloan_precompile_address = H160::from_low_u64_be(2049);

			// Construct the input data to check if Bob is a contributor
			let mut bob_input_data = Vec::<u8>::from([0u8; 36]);
			bob_input_data[0..4]
				.copy_from_slice(&Keccak256::digest(b"is_contributor(address)")[0..4]);
			bob_input_data[16..36].copy_from_slice(&BOB);

			// Expected result is an EVM boolean false which is 256 bits long.
			let mut expected_bytes = Vec::from([0u8; 32]);
			expected_bytes[31] = 0;
			let expected_false_result = Some(Ok(PrecompileOutput {
				exit_status: ExitSucceed::Returned,
				output: expected_bytes,
				cost: 1000,
				logs: Default::default(),
			}));

			// Assert precompile reports Bob is not a contributor
			assert_eq!(
				Precompiles::execute(
					crowdloan_precompile_address,
					&bob_input_data,
					None, // target_gas is not necessary right now because consumed none now
					&Context {
						// This context copied from Sacrifice tests, it's not great.
						address: Default::default(),
						caller: Default::default(),
						apparent_value: From::from(0),
					}
				),
				expected_false_result
			);

			// Construct the input data to check if Charlie is a contributor
			let mut charlie_input_data = Vec::<u8>::from([0u8; 36]);
			charlie_input_data[0..4]
				.copy_from_slice(&Keccak256::digest(b"is_contributor(address)")[0..4]);
			charlie_input_data[16..36].copy_from_slice(&CHARLIE);

			// Expected result is an EVM boolean true which is 256 bits long.
			let mut expected_bytes = Vec::from([0u8; 32]);
			expected_bytes[31] = 1;
			let expected_true_result = Some(Ok(PrecompileOutput {
				exit_status: ExitSucceed::Returned,
				output: expected_bytes,
				cost: 1000,
				logs: Default::default(),
			}));

			// Assert precompile reports Bob is a nominator
			assert_eq!(
				Precompiles::execute(
					crowdloan_precompile_address,
					&charlie_input_data,
					None, // target_gas is not necessary right now because consumed none now
					&Context {
						// This context copied from Sacrifice tests, it's not great.
						address: Default::default(),
						caller: Default::default(),
						apparent_value: From::from(0),
					}
				),
				expected_true_result
			);
		})
}

#[test]
fn reward_info_via_precompile() {
	ExtBuilder::default()
		.with_balances(vec![
			(AccountId::from(ALICE), 2_000 * MOVR),
			(AccountId::from(BOB), 1_000 * MOVR),
		])
		.with_collators(vec![(AccountId::from(ALICE), 1_000 * MOVR)])
		.with_mappings(vec![(
			NimbusId::from_slice(&ALICE_NIMBUS),
			AccountId::from(ALICE),
		)])
		.with_crowdloan_fund(3_000_000 * MOVR)
		.build()
		.execute_with(|| {
			// set parachain inherent data
			set_parachain_inherent_data();
			set_author(NimbusId::from_slice(&ALICE_NIMBUS));
			for x in 1..3 {
				run_to_block(x);
			}
			let init_block = CrowdloanRewards::init_relay_block();
			// This matches the previous vesting
			let end_block = init_block + 4 * WEEKS;
			// Batch calls always succeed. We just need to check the inner event
			assert_ok!(
				Call::Utility(pallet_utility::Call::<Runtime>::batch_all(vec![
					Call::CrowdloanRewards(
						pallet_crowdloan_rewards::Call::<Runtime>::initialize_reward_vec(vec![(
							[4u8; 32].into(),
							Some(AccountId::from(CHARLIE)),
							1_500_000 * MOVR
						)])
					),
					Call::CrowdloanRewards(
						pallet_crowdloan_rewards::Call::<Runtime>::initialize_reward_vec(vec![(
							[5u8; 32].into(),
							Some(AccountId::from(DAVE)),
							1_500_000 * MOVR
						)])
					),
					Call::CrowdloanRewards(
						pallet_crowdloan_rewards::Call::<Runtime>::complete_initialization(
							end_block
						)
					)
				]))
				.dispatch(root_origin())
			);

			let crowdloan_precompile_address = H160::from_low_u64_be(2049);

			// Construct the input data to check if Bob is a contributor
			let mut charlie_input_data = Vec::<u8>::from([0u8; 36]);
			charlie_input_data[0..4]
				.copy_from_slice(&Keccak256::digest(b"reward_info(address)")[0..4]);
			charlie_input_data[16..36].copy_from_slice(&CHARLIE);

			let expected_total: U256 = (1_500_000 * MOVR).into();
			let expected_claimed: U256 = (450_000 * MOVR).into();

			// Expected result is two EVM u256 false which are 256 bits long.
			let mut expected_bytes = Vec::from([0u8; 64]);
			expected_total.to_big_endian(&mut expected_bytes[0..32]);
			expected_claimed.to_big_endian(&mut expected_bytes[32..64]);
			let expected_result = Some(Ok(PrecompileOutput {
				exit_status: ExitSucceed::Returned,
				output: expected_bytes,
				cost: 1000,
				logs: Default::default(),
			}));

			// Assert precompile reports Bob is not a contributor
			assert_eq!(
				Precompiles::execute(
					crowdloan_precompile_address,
					&charlie_input_data,
					None, // target_gas is not necessary right now because consumed none now
					&Context {
						// This context copied from Sacrifice tests, it's not great.
						address: Default::default(),
						caller: Default::default(),
						apparent_value: From::from(0),
					},
				),
				expected_result
			);
		})
}

#[test]
fn update_reward_address_via_precompile() {
	ExtBuilder::default()
		.with_balances(vec![
			(AccountId::from(ALICE), 2_000 * MOVR),
			(AccountId::from(BOB), 1_000 * MOVR),
		])
		.with_collators(vec![(AccountId::from(ALICE), 1_000 * MOVR)])
		.with_mappings(vec![(
			NimbusId::from_slice(&ALICE_NIMBUS),
			AccountId::from(ALICE),
		)])
		.with_crowdloan_fund(3_000_000 * MOVR)
		.build()
		.execute_with(|| {
			// set parachain inherent data
			set_parachain_inherent_data();
			set_author(NimbusId::from_slice(&ALICE_NIMBUS));
			for x in 1..3 {
				run_to_block(x);
			}
			let init_block = CrowdloanRewards::init_relay_block();
			// This matches the previous vesting
			let end_block = init_block + 4 * WEEKS;
			// Batch calls always succeed. We just need to check the inner event
			assert_ok!(
				Call::Utility(pallet_utility::Call::<Runtime>::batch_all(vec![
					Call::CrowdloanRewards(
						pallet_crowdloan_rewards::Call::<Runtime>::initialize_reward_vec(vec![(
							[4u8; 32].into(),
							Some(AccountId::from(CHARLIE)),
							1_500_000 * MOVR
						)])
					),
					Call::CrowdloanRewards(
						pallet_crowdloan_rewards::Call::<Runtime>::initialize_reward_vec(vec![(
							[5u8; 32].into(),
							Some(AccountId::from(DAVE)),
							1_500_000 * MOVR
						)])
					),
					Call::CrowdloanRewards(
						pallet_crowdloan_rewards::Call::<Runtime>::complete_initialization(
							end_block
						)
					)
				]))
				.dispatch(root_origin())
			);

			let crowdloan_precompile_address = H160::from_low_u64_be(2049);

			// Charlie uses the crowdloan precompile to update address through the EVM
			let gas_limit = 100000u64;
			let gas_price: U256 = 1_000_000_000.into();

			// Construct the input data to check if Bob is a contributor
			let mut call_data = Vec::<u8>::from([0u8; 36]);
			call_data[0..4]
				.copy_from_slice(&Keccak256::digest(b"update_reward_address(address)")[0..4]);
			call_data[16..36].copy_from_slice(&ALICE);

			assert_ok!(Call::EVM(pallet_evm::Call::<Runtime>::call(
				AccountId::from(CHARLIE),
				crowdloan_precompile_address,
				call_data,
				U256::zero(), // No value sent in EVM
				gas_limit,
				gas_price,
				None, // Use the next nonce
			))
			.dispatch(<Runtime as frame_system::Config>::Origin::root()));

			assert!(CrowdloanRewards::accounts_payable(&AccountId::from(CHARLIE)).is_none());
			assert_eq!(
				CrowdloanRewards::accounts_payable(&AccountId::from(ALICE))
					.unwrap()
					.claimed_reward,
				(450_000 * MOVR)
			);
		})
}

fn run_with_system_weight<F>(w: Weight, mut assertions: F)
where
	F: FnMut() -> (),
{
	let mut t: sp_io::TestExternalities = frame_system::GenesisConfig::default()
		.build_storage::<Runtime>()
		.unwrap()
		.into();
	t.execute_with(|| {
		System::set_block_consumed_resources(w, 0);
		assertions()
	});
}

#[test]
fn multiplier_can_grow_from_zero() {
	let minimum_multiplier = moonriver_runtime::MinimumMultiplier::get();
	let target = moonriver_runtime::TargetBlockFullness::get()
		* BlockWeights::get()
			.get(DispatchClass::Normal)
			.max_total
			.unwrap();
	// if the min is too small, then this will not change, and we are doomed forever.
	// the weight is 1/100th bigger than target.
	run_with_system_weight(target * 101 / 100, || {
		let next =
			moonriver_runtime::SlowAdjustingFeeUpdate::<Runtime>::convert(minimum_multiplier);
		assert!(
			next > minimum_multiplier,
			"{:?} !>= {:?}",
			next,
			minimum_multiplier
		);
	})
}

#[test]
#[ignore] // test runs for a very long time
fn multiplier_growth_simulator() {
	// assume the multiplier is initially set to its minimum. We update it with values twice the
	//target (target is 25%, thus 50%) and we see at which point it reaches 1.
	let mut multiplier = moonriver_runtime::MinimumMultiplier::get();
	let block_weight = moonriver_runtime::TargetBlockFullness::get()
		* BlockWeights::get()
			.get(DispatchClass::Normal)
			.max_total
			.unwrap()
		* 2;
	let mut blocks = 0;
	while multiplier <= Multiplier::one() {
		run_with_system_weight(block_weight, || {
			let next = moonriver_runtime::SlowAdjustingFeeUpdate::<Runtime>::convert(multiplier);
			// ensure that it is growing as well.
			assert!(next > multiplier, "{:?} !>= {:?}", next, multiplier);
			multiplier = next;
		});
		blocks += 1;
		println!("block = {} multiplier {:?}", blocks, multiplier);
	}
}

#[test]
fn ethereum_invalid_transaction() {
	ExtBuilder::default().build().execute_with(|| {
		// Ensure an extrinsic not containing enough gas limit to store the transaction
		// on chain is rejected.
		assert_eq!(
			Executive::apply_extrinsic(unchecked_eth_tx(INVALID_ETH_TX)),
			Err(
				sp_runtime::transaction_validity::TransactionValidityError::Invalid(
					sp_runtime::transaction_validity::InvalidTransaction::Custom(3u8)
				)
			)
		);
	});
}
