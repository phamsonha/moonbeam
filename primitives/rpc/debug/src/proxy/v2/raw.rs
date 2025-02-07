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

extern crate alloc;
use super::{
	convert_memory, Capture, ContextType, Event, ExitReason, GasometerEvent, Listener as ListenerT,
	RuntimeEvent, H160, H256,
};
use alloc::{collections::btree_map::BTreeMap, vec, vec::Vec};

use crate::single::RawStepLog;

#[derive(Debug)]
pub struct Listener {
	disable_storage: bool,
	disable_memory: bool,
	disable_stack: bool,

	new_context: bool,
	context_stack: Vec<Context>,

	pub step_logs: Vec<RawStepLog>,
	pub return_value: Vec<u8>,
	pub final_gas: u64,
}

#[derive(Debug)]
struct Context {
	storage_cache: BTreeMap<H256, H256>,
	address: H160,
	current_step: Option<Step>,
	global_storage_changes: BTreeMap<H160, BTreeMap<H256, H256>>,
}

#[derive(Debug)]
struct Step {
	/// Current opcode.
	opcode: Vec<u8>,
	/// Depth of the context.
	depth: usize,
	/// Remaining gas.
	gas: u64,
	/// Gas cost of the following opcode.
	gas_cost: u64,
	/// Program counter position.
	position: usize,
	/// EVM memory copy (if not disabled).
	memory: Option<Vec<u8>>,
	/// EVM stack copy (if not disabled).
	stack: Option<Vec<H256>>,
}

impl Listener {
	pub fn new(disable_storage: bool, disable_memory: bool, disable_stack: bool) -> Self {
		Self {
			disable_storage,
			disable_memory,
			disable_stack,

			step_logs: vec![],
			return_value: vec![],
			final_gas: 0,

			new_context: false,
			context_stack: vec![],
		}
	}

	pub fn using<R, F: FnOnce() -> R>(&mut self, f: F) -> R {
		super::listener::using(self, f)
	}

	pub fn gasometer_event(&mut self, event: GasometerEvent) {
		match event {
			GasometerEvent::RecordTransaction { .. } => {
				// First event of a transaction.
				// Next step will be the first context.
				self.new_context = true;
			}
			GasometerEvent::RecordCost { cost, snapshot } => {
				if let Some(context) = self.context_stack.last_mut() {
					// Register opcode cost. (ignore costs not between Step and StepResult)
					if let Some(step) = &mut context.current_step {
						step.gas = snapshot.gas();
						step.gas_cost = cost;
						self.final_gas = step.gas;
					}
				}
			}
			GasometerEvent::RecordDynamicCost {
				gas_cost, snapshot, ..
			} => {
				if let Some(context) = self.context_stack.last_mut() {
					// Register opcode cost. (ignore costs not between Step and StepResult)
					if let Some(step) = &mut context.current_step {
						step.gas = snapshot.gas();
						step.gas_cost = gas_cost;
						self.final_gas = step.gas;
					}
				}
			}
			// We ignore other kinds of message if any (new ones may be added in the future).
			#[allow(unreachable_patterns)]
			_ => (),
		}
	}

	pub fn runtime_event(&mut self, event: RuntimeEvent) {
		match event {
			RuntimeEvent::Step {
				context,
				opcode,
				position,
				stack,
				memory,
			} => {
				// Create a context if needed.
				if self.new_context {
					self.new_context = false;

					self.context_stack.push(Context {
						storage_cache: BTreeMap::new(),
						address: context.address,
						current_step: None,
						global_storage_changes: BTreeMap::new(),
					});
				}

				let depth = self.context_stack.len();

				// Ignore steps outside of any context (shouldn't even be possible).
				if let Some(context) = self.context_stack.last_mut() {
					context.current_step = Some(Step {
						opcode,
						depth,
						gas: 0,      // 0 for now, will add with gas events
						gas_cost: 0, // 0 for now, will add with gas events
						position: *position.as_ref().unwrap_or(&0) as usize,
						memory: if self.disable_memory {
							None
						} else {
							Some(memory.data.clone())
						},
						stack: if self.disable_stack {
							None
						} else {
							Some(stack.data.clone())
						},
					});
				}
			}
			RuntimeEvent::StepResult {
				result,
				return_value,
			} => {
				// StepResult is expected to be emited after a step (in a context).
				// Only case StepResult will occur without a Step before is in a transfer
				// transaction to a non-contract address. However it will not contain any
				// steps and return an empty trace, so we can ignore this edge case.
				if let Some(context) = self.context_stack.last_mut() {
					if let Some(current_step) = context.current_step.take() {
						let Step {
							opcode,
							depth,
							gas,
							gas_cost,
							position,
							memory,
							stack,
						} = current_step;

						let memory = memory.map(convert_memory);

						let storage = if self.disable_storage {
							None
						} else {
							Some(context.storage_cache.clone())
						};

						self.step_logs.push(RawStepLog {
							depth: depth.into(),
							gas: gas.into(),
							gas_cost: gas_cost.into(),
							memory,
							op: opcode,
							pc: position.into(),
							stack,
							storage,
						});
					}
				}

				// We match on the capture to handle traps/exits.
				match result {
					Err(Capture::Exit(reason)) => {
						// Exit = we exit the context (should always be some)
						if let Some(mut context) = self.context_stack.pop() {
							// If final context is exited, we store gas and return value.
							if self.context_stack.is_empty() {
								self.return_value = return_value.to_vec();
							}

							// If the context exited without revert we must keep track of the
							// updated storage keys.
							if !self.disable_storage && matches!(reason, ExitReason::Succeed(_)) {
								if let Some(parent_context) = self.context_stack.last_mut() {
									// Add cache to storage changes.
									context
										.global_storage_changes
										.insert(context.address, context.storage_cache);

									// Apply storage changes to parent, either updating its cache or map of changes.
									for (address, mut storage) in
										context.global_storage_changes.into_iter()
									{
										// Same address => We update its cache (only tracked keys)
										if parent_context.address == address {
											for (cached_key, cached_value) in
												parent_context.storage_cache.iter_mut()
											{
												if let Some(value) = storage.remove(cached_key) {
													*cached_value = value;
												}
											}
										}
										// Otherwise, update the storage changes.
										else {
											parent_context
												.global_storage_changes
												.entry(address)
												.or_insert_with(BTreeMap::new)
												.append(&mut storage);
										}
									}
								}
							}
						}
					}
					Err(Capture::Trap(opcode)) if ContextType::from(opcode.clone()).is_some() => {
						self.new_context = true;
					}
					_ => (),
				}
			}
			RuntimeEvent::SLoad {
				address: _,
				index,
				value,
			}
			| RuntimeEvent::SStore {
				address: _,
				index,
				value,
			} => {
				if let Some(context) = self.context_stack.last_mut() {
					if !self.disable_storage {
						context.storage_cache.insert(index, value);
					}
				}
			}
			// We ignore other kinds of message if any (new ones may be added in the future).
			#[allow(unreachable_patterns)]
			_ => (),
		}
	}
}

impl ListenerT for Listener {
	fn event(&mut self, event: Event) {
		match event {
			Event::Gasometer(gasometer_event) => self.gasometer_event(gasometer_event),
			Event::Runtime(runtime_event) => self.runtime_event(runtime_event),
			_ => {}
		};
	}
}
