
//! Autogenerated weights for pallet_funding
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-12-23, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `leoair.lan`, CPU: `<UNKNOWN>`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/polimec-standalone-node
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_funding
// --extrinsic
// *
// --execution=wasm
// --heap-pages=4096
// --output=pallets/funding/src/weights.rs
// --template=./.maintain/frame-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_funding.
pub trait WeightInfo {
	fn create() -> Weight;
	fn start_evaluation() -> Weight;
	fn on_finalize() -> Weight;
}

/// Weights for pallet_funding using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: PolimecFunding ProjectId (r:1 w:1)
	// Storage: PolimecFunding Projects (r:0 w:1)
	// Storage: PolimecFunding ProjectsInfo (r:0 w:1)
	// Storage: PolimecFunding ProjectsIssuers (r:0 w:1)
	fn create() -> Weight {
		// Minimum execution time:  nanoseconds.
		Weight::from_ref_time(22_000_000)
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	// Storage: PolimecFunding ProjectsIssuers (r:1 w:0)
	// Storage: PolimecFunding ProjectsInfo (r:1 w:1)
	// Storage: PolimecFunding ProjectsActive (r:1 w:1)
	fn start_evaluation() -> Weight {
		// Minimum execution time:  nanoseconds.
		Weight::from_ref_time(26_000_000)
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	// Storage: PolimecFunding ProjectsActive (r:1 w:0)
	// Storage: PolimecFunding ProjectsInfo (r:100 w:100)
	fn on_finalize() -> Weight {
		// Minimum execution time:  nanoseconds.
		Weight::from_ref_time(814_000_000)
			.saturating_add(T::DbWeight::get().reads(101))
			.saturating_add(T::DbWeight::get().writes(100))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: PolimecFunding ProjectId (r:1 w:1)
	// Storage: PolimecFunding Projects (r:0 w:1)
	// Storage: PolimecFunding ProjectsInfo (r:0 w:1)
	// Storage: PolimecFunding ProjectsIssuers (r:0 w:1)
	fn create() -> Weight {
		// Minimum execution time:  nanoseconds.
		Weight::from_ref_time(22_000_000)
			.saturating_add(RocksDbWeight::get().reads(1))
			.saturating_add(RocksDbWeight::get().writes(4))
	}
	// Storage: PolimecFunding ProjectsIssuers (r:1 w:0)
	// Storage: PolimecFunding ProjectsInfo (r:1 w:1)
	// Storage: PolimecFunding ProjectsActive (r:1 w:1)
	fn start_evaluation() -> Weight {
		// Minimum execution time:  nanoseconds.
		Weight::from_ref_time(26_000_000)
			.saturating_add(RocksDbWeight::get().reads(3))
			.saturating_add(RocksDbWeight::get().writes(2))
	}
	// Storage: PolimecFunding ProjectsActive (r:1 w:0)
	// Storage: PolimecFunding ProjectsInfo (r:100 w:100)
	fn on_finalize() -> Weight {
		// Minimum execution time:  nanoseconds.
		Weight::from_ref_time(814_000_000)
			.saturating_add(RocksDbWeight::get().reads(101))
			.saturating_add(RocksDbWeight::get().writes(100))
	}
}