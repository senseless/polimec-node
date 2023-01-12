#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

mod weights;
pub mod xcm_config;

use cumulus_pallet_parachain_system::RelayNumberStrictlyIncreases;
use frame_support::{
	construct_runtime,
	dispatch::DispatchClass,
	pallet_prelude::Get,
	parameter_types,
	traits::{
		ConstU32, Currency, EitherOfDiverse, EqualPrivilegeOnly, Everything, Imbalance,
		OnUnbalanced,
	},
	weights::{
		ConstantMultiplier, Weight, WeightToFee as WeightToFeeT, WeightToFeeCoefficient,
		WeightToFeeCoefficients, WeightToFeePolynomial,
	},
	PalletId,
};
use frame_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot,
};

use pallet_balances::WeightInfo;
use pallet_transaction_payment::OnChargeTransaction;
pub use parachain_staking::InflationInfo;

pub use parachains_common::{
	impls::{AssetsToBlockAuthor, DealWithFees},
	opaque, AccountId, AssetId, AuraId, Balance, BlockNumber, Hash, Header, Index, Signature,
	AVERAGE_ON_INITIALIZE_RATIO, DAYS, HOURS, MAXIMUM_BLOCK_WEIGHT, MINUTES, NORMAL_DISPATCH_RATIO,
	SLOT_DURATION,
};

use polimec_traits::{MemberRole, PolimecMembers};
use smallvec::smallvec;
use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{AccountIdLookup, BlakeTwo256, Block as BlockT, ConvertInto, OpaqueKeys},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, Perquintill,
};
pub use sp_runtime::{MultiAddress, Perbill, Permill};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
use xcm_config::XcmConfig;

#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

// Polkadot imports
use polkadot_runtime_common::{BlockHashCount, SlowAdjustingFeeUpdate};

use weights::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight};

// XCM Imports
use xcm::latest::prelude::BodyId;
use xcm_executor::XcmExecutor;

pub type Amount = i128;

pub type CurrencyId = [u8; 8];

pub type NegativeImbalanceOf<T> = <pallet_balances::Pallet<T> as Currency<
	<T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

/// The address format for describing accounts.
pub type Address = MultiAddress<AccountId, ()>;

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;

/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;

/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	// pallet_asset_tx_payment::ChargeAssetTxPayment<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;

/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, RuntimeCall, SignedExtra>;

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

/// Handles converting a weight scalar to a fee value, based on the scale and
/// granularity of the node's balance type.
///
/// This should typically create a mapping between the following ranges:
///   - [0, MAXIMUM_BLOCK_WEIGHT]
///   - [Balance::min, Balance::max]
///
/// Yet, it can be used for any other sort of change to weight-fee. Some
/// examples being:
///   - Setting it to `0` will essentially disable the weight fee.
///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
pub struct WeightToFee<R>(sp_std::marker::PhantomData<R>);
impl<R> WeightToFeePolynomial for WeightToFee<R>
where
	R: pallet_transaction_payment::Config,
	R: frame_system::Config,
	R: pallet_balances::Config,
	u128: From<
		<<R as pallet_transaction_payment::Config>::OnChargeTransaction as OnChargeTransaction<
			R,
		>>::Balance,
	>,
{
	type Balance = Balance;
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		// The should be fee
		let wanted_fee: Balance = 10 * MILLI_PLMC;

		// TODO: transfer_keep_alive is 288 byte long?
		let tx_len: u64 = 288;
		let byte_fee: Balance =
			<R as pallet_transaction_payment::Config>::LengthToFee::weight_to_fee(
				&Weight::from_ref_time(tx_len),
			)
			.into();
		let base_weight: Weight = <R as frame_system::Config>::BlockWeights::get()
			.get(DispatchClass::Normal)
			.base_extrinsic;
		let base_weight_fee: Balance =
			<R as pallet_transaction_payment::Config>::LengthToFee::weight_to_fee(&base_weight)
				.into();
		let tx_weight_fee: Balance =
			<R as pallet_transaction_payment::Config>::LengthToFee::weight_to_fee(
				&<R as pallet_balances::Config>::WeightInfo::transfer_keep_alive(),
			)
			.into();
		let unbalanced_fee: Balance = base_weight_fee.saturating_add(tx_weight_fee);

		let wanted_weight_fee: Balance = wanted_fee.saturating_sub(byte_fee);

		smallvec![WeightToFeeCoefficient {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(wanted_weight_fee % unbalanced_fee, unbalanced_fee),
			coeff_integer: wanted_weight_fee / unbalanced_fee,
		}]
	}
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
	}
}

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("polimec-node"),
	impl_name: create_runtime_str!("polimec-node"),
	authoring_version: 1,
	spec_version: 1,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

/// This determines the average expected block time that we are targeting.
/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
/// up by `pallet_aura` to implement `fn slot_duration()`.
///
/// Change this to adjust the block time.
pub const MILLISECS_PER_BLOCK: u64 = 12_000;

// Time is measured by number of blocks.
pub const BLOCKS_PER_YEAR: BlockNumber = DAYS * 36525 / 100;
pub const INITIAL_PERIOD_LENGTH: BlockNumber = BLOCKS_PER_YEAR.saturating_mul(5);

// Unit = the base number of indivisible units for balances
pub const PLMC: Balance = 1_000_000_000_000;
pub const MILLI_PLMC: Balance = 1_000_000_000;
pub const MICRO_PLMC: Balance = 1_000_000;

/// The existential deposit. Set to 1/10 of the Connected Relay Chain.
pub const EXISTENTIAL_DEPOSIT: Balance = MILLI_PLMC;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;

	// This part is copied from Substrate's `bin/node/runtime/src/lib.rs`.
	//  The `RuntimeBlockLength` and `RuntimeBlockWeights` exist here because the
	// `DeletionWeightLimit` and `DeletionQueueDepth` depend on those to parameterize
	// the lazy contract deletion.
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
	pub const SS58Prefix: u16 = 41;
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Config for Runtime {
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, ()>;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Index;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// Runtime version.
	type Version = Version;
	/// Converts a module to an index of this module in the runtime.
	type PalletInfo = PalletInfo;
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = Everything;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = RuntimeBlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = RuntimeBlockLength;
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	/// The action to take on a Runtime Upgrade
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Runtime>;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const UncleGenerations: u32 = 0;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
	type UncleGenerations = UncleGenerations;
	type FilterUncle = ();
	type EventHandler = ParachainStaking;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = Treasury;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
}

/// Logic for the author to get a portion of fees.
pub struct ToAuthor<R>(sp_std::marker::PhantomData<R>);

impl<R> OnUnbalanced<NegativeImbalanceOf<R>> for ToAuthor<R>
where
	R: pallet_balances::Config + pallet_authorship::Config,
	<R as frame_system::Config>::AccountId: From<AccountId>,
	<R as frame_system::Config>::AccountId: Into<AccountId>,
	<R as pallet_balances::Config>::Balance: Into<u128>,
{
	fn on_nonzero_unbalanced(amount: NegativeImbalanceOf<R>) {
		if let Some(author) = <pallet_authorship::Pallet<R>>::author() {
			<pallet_balances::Pallet<R>>::resolve_creating(&author, amount);
		}
	}
}

/// Split two Imbalances between two unbalanced handlers.
/// The first Imbalance will be split according to the given ratio. The second
/// Imbalance will be handled by the second beneficiary.
///
/// In case of transaction payment, the first Imbalance is the fee and the
/// second imbalance the tip.
pub struct SplitFeesByRatio<R, Ratio, Beneficiary1, Beneficiary2>(
	sp_std::marker::PhantomData<(R, Ratio, Beneficiary1, Beneficiary2)>,
);
impl<R, Ratio, Beneficiary1, Beneficiary2> OnUnbalanced<NegativeImbalanceOf<R>>
	for SplitFeesByRatio<R, Ratio, Beneficiary1, Beneficiary2>
where
	R: pallet_balances::Config,
	Beneficiary1: OnUnbalanced<NegativeImbalanceOf<R>>,
	Beneficiary2: OnUnbalanced<NegativeImbalanceOf<R>>,
	Ratio: Get<(u32, u32)>,
{
	fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalanceOf<R>>) {
		let ratio = Ratio::get();
		if let Some(fees) = fees_then_tips.next() {
			let mut split = fees.ration(ratio.0, ratio.1);
			if let Some(tips) = fees_then_tips.next() {
				// for tips, if any, 100% to author
				tips.merge_into(&mut split.1);
			}
			Beneficiary1::on_unbalanced(split.0);
			Beneficiary2::on_unbalanced(split.1);
		}
	}
}

parameter_types! {
	/// Relay Chain `TransactionByteFee` / 10
	pub const TransactionByteFee: Balance = 10 * MICRO_PLMC;
	pub const OperationalFeeMultiplier: u8 = 5;
	pub const FeeSplitRatio: (u32, u32) = (50, 50);
}

/// Split the fees using a preconfigured Ratio
pub type FeeSplit<R, B1, B2> = SplitFeesByRatio<R, FeeSplitRatio, B1, B2>;

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<
		Balances,
		FeeSplit<Runtime, Treasury, ToAuthor<Runtime>>,
	>;
	type WeightToFee = WeightToFee<Runtime>;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
}

impl pallet_asset_tx_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Fungibles = Assets;
	type OnChargeAssetTransaction = pallet_asset_tx_payment::FungiblesAdapter<
		pallet_assets::BalanceToAssetBalance<Balances, Runtime, ConvertInto>,
		AssetsToBlockAuthor<Runtime>,
	>;
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
}

// TODO: On Testnet only
impl pallet_sudo::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnSystemEvent = ();
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type DmpMessageHandler = DmpQueue;
	type ReservedDmpWeight = ReservedDmpWeight;
	type OutboundXcmpMessageSource = XcmpQueue;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type CheckAssociatedRelayNumber = RelayNumberStrictlyIncreases;
}

impl parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ChannelInfo = ParachainSystem;
	type VersionWrapper = PolkadotXcm;
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = xcm_config::XcmOriginToTransactDispatchOrigin;
	type WeightInfo = ();
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
}

parameter_types! {
	pub const Period: BlockNumber = 6 * HOURS;
	pub const Offset: BlockNumber = 0;
	pub const MaxAuthorities: u32 = 100_000;
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = ConvertInto;
	type ShouldEndSession = ParachainStaking;
	type NextSessionRotation = ParachainStaking;
	type SessionManager = ParachainStaking;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type WeightInfo = ();
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	// TODO: handle disabled validators
	type DisabledValidators = ();
	type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
	pub const PotId: PalletId = PalletId(*b"PotStake");
	pub const MaxCandidates: u32 = 1000;
	pub const MinCandidates: u32 = 5;
	pub const SessionLength: BlockNumber = 6 * HOURS;
	pub const MaxInvulnerables: u32 = 100;
	pub const ExecutiveBody: BodyId = BodyId::Executive;
}

/// Minimum round length is 1 hour (300 * 12 second block times)
#[cfg(feature = "fast-gov")]
pub const MIN_BLOCKS_PER_ROUND: BlockNumber = 10;
#[cfg(not(feature = "fast-gov"))]
pub const MIN_BLOCKS_PER_ROUND: BlockNumber = HOURS;

#[cfg(feature = "fast-gov")]
pub const DEFAULT_BLOCKS_PER_ROUND: BlockNumber = 20;
#[cfg(not(feature = "fast-gov"))]
pub const DEFAULT_BLOCKS_PER_ROUND: BlockNumber = 2 * HOURS;

#[cfg(feature = "fast-gov")]
pub const STAKE_DURATION: BlockNumber = 30;
#[cfg(not(feature = "fast-gov"))]
pub const STAKE_DURATION: BlockNumber = 7 * DAYS;

#[cfg(feature = "fast-gov")]
pub const MIN_COLLATORS: u32 = 4;
#[cfg(not(feature = "fast-gov"))]
pub const MIN_COLLATORS: u32 = 16;

#[cfg(feature = "fast-gov")]
pub const MAX_CANDIDATES: u32 = 16;
#[cfg(not(feature = "fast-gov"))]
pub const MAX_CANDIDATES: u32 = 75;

pub const MAX_DELEGATORS_PER_COLLATOR: u32 = 35;
pub const MIN_DELEGATOR_STAKE: Balance = 20 * PLMC;

pub const NETWORK_REWARD_RATE: Perquintill = Perquintill::from_percent(10);

parameter_types! {
	/// Minimum round length is 1 hour
	pub const MinBlocksPerRound: BlockNumber = MIN_BLOCKS_PER_ROUND;
	/// Default length of a round/session is 2 hours
	pub const DefaultBlocksPerRound: BlockNumber = DEFAULT_BLOCKS_PER_ROUND;
	/// Unstaked balance can be unlocked after 7 days
	pub const StakeDuration: BlockNumber = STAKE_DURATION;
	/// Collator exit requests are delayed by 4 hours (2 rounds/sessions)
	pub const ExitQueueDelay: u32 = 2;
	/// Minimum 16 collators selected per round, default at genesis and minimum forever after
	pub const MinCollators: u32 = MIN_COLLATORS;
	/// At least 4 candidates which cannot leave the network if there are no other candidates.
	pub const MinRequiredCollators: u32 = 4;
	/// We only allow one delegation per round.
	pub const MaxDelegationsPerRound: u32 = 1;
	/// Maximum 25 delegators per collator at launch, might be increased later
	#[derive(Debug, Eq, PartialEq)]
	pub const MaxDelegatorsPerCollator: u32 = MAX_DELEGATORS_PER_COLLATOR;
	/// Minimum stake required to be reserved to be a collator is 10_000
	pub const MinCollatorStake: Balance = 10_000 * PLMC;
	/// Minimum stake required to be reserved to be a delegator is 1000
	pub const MinDelegatorStake: Balance = MIN_DELEGATOR_STAKE;
	/// Maximum number of collator candidates
	#[derive(Debug, Eq, PartialEq)]
	pub const MaxCollatorCandidates: u32 = MAX_CANDIDATES;
	/// Maximum number of concurrent requests to unlock unstaked balance
	pub const MaxUnstakeRequests: u32 = 10;
	/// The starting block number for the network rewards
	pub const NetworkRewardStart: BlockNumber = INITIAL_PERIOD_LENGTH;
	/// The rate in percent for the network rewards
	pub const NetworkRewardRate: Perquintill = NETWORK_REWARD_RATE;
}

impl parachain_staking::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type CurrencyBalance = Balance;

	type MinBlocksPerRound = MinBlocksPerRound;
	type DefaultBlocksPerRound = DefaultBlocksPerRound;
	type StakeDuration = StakeDuration;
	type ExitQueueDelay = ExitQueueDelay;
	type MinCollators = MinCollators;
	type MinRequiredCollators = MinRequiredCollators;
	type MaxDelegationsPerRound = MaxDelegationsPerRound;
	type MaxDelegatorsPerCollator = MaxDelegatorsPerCollator;
	type MinCollatorStake = MinCollatorStake;
	type MinCollatorCandidateStake = MinCollatorStake;
	type MaxTopCandidates = MaxCollatorCandidates;
	type MinDelegatorStake = MinDelegatorStake;
	type MaxUnstakeRequests = MaxUnstakeRequests;
	type NetworkRewardRate = NetworkRewardRate;
	type NetworkRewardStart = NetworkRewardStart;
	type NetworkRewardBeneficiary = ();
	// type NetworkRewardBeneficiary = Treasury;
	type WeightInfo = ();

	const BLOCKS_PER_YEAR: Self::BlockNumber = BLOCKS_PER_YEAR;
}

impl pallet_randomness_collective_flip::Config for Runtime {}

#[cfg(feature = "fast-gov")]
pub const EVALUATION_DURATION: BlockNumber = 10;
#[cfg(not(feature = "fast-gov"))]
pub const EVALUATION_DURATION: BlockNumber = 28 * DAYS;

#[cfg(feature = "fast-gov")]
pub const ENGLISH_AUCTION_DURATION: BlockNumber = 10;
#[cfg(not(feature = "fast-gov"))]
pub const ENGLISH_AUCTION_DURATION: BlockNumber = 5 * DAYS;

#[cfg(feature = "fast-gov")]
pub const CANDLE_AUCTION_DURATION: BlockNumber = 10;
#[cfg(not(feature = "fast-gov"))]
pub const CANDLE_AUCTION_DURATION: BlockNumber = 12 * HOURS;

#[cfg(feature = "fast-gov")]
pub const COMMUNITY_DURATION: BlockNumber = 10;
#[cfg(not(feature = "fast-gov"))]
pub const COMMUNITY_DURATION: BlockNumber = 5 * DAYS;

parameter_types! {
	pub const EvaluationDuration: BlockNumber = EVALUATION_DURATION;
	pub const EnglishAuctionDuration: BlockNumber = ENGLISH_AUCTION_DURATION;
	pub const CandleAuctionDuration: BlockNumber = CANDLE_AUCTION_DURATION;
	pub const CommunityRoundDuration: BlockNumber = COMMUNITY_DURATION;
	pub const FundingPalletId: PalletId = PalletId(*b"py/cfund");
}

impl pallet_funding::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type StringLimit = ConstU32<64>;
	type Currency = Balances;
	type ProjectIdentifier = u32;
	type BiddingCurrency = Balances;
	type CurrencyBalance = <Self as pallet_balances::Config>::Balance;
	type EvaluationDuration = EvaluationDuration;
	type PalletId = FundingPalletId;
	type ActiveProjectsLimit = ConstU32<100>;
	type EnglishAuctionDuration = EnglishAuctionDuration;
	type CandleAuctionDuration = CandleAuctionDuration;
	type CommunityRoundDuration = CommunityRoundDuration;
	type Randomness = Random;
	type HandleMembers = Credentials;
	type MaximumBidsPerProject = ConstU32<256>;
	type WeightInfo = weights::pallet_funding::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

impl pallet_credentials::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AddOrigin = EnsureRoot<AccountId>;
	type RemoveOrigin = EnsureRoot<AccountId>;
	type SwapOrigin = EnsureRoot<AccountId>;
	type ResetOrigin = EnsureRoot<AccountId>;
	type PrimeOrigin = EnsureRoot<AccountId>;
	type MembershipInitialized = ();
	type MembershipChanged = ();
}

#[cfg(feature = "fast-gov")]
pub const SPEND_PERIOD: BlockNumber = 6 * MINUTES;
#[cfg(not(feature = "fast-gov"))]
pub const SPEND_PERIOD: BlockNumber = 6 * DAYS;

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = 20 * PLMC;
	pub const SpendPeriod: BlockNumber = SPEND_PERIOD;
	pub const Burn: Permill = Permill::zero();
	pub const MaxApprovals: u32 = 100;
	pub const TreasuryPalletId: PalletId = PalletId(*b"politrea");
}

impl pallet_treasury::Config for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = Balances;
	// TODO: Use the Council instead of Root!
	type ApproveOrigin = EnsureRoot<AccountId>;
	type RejectOrigin = EnsureRoot<AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type OnSlash = Treasury;
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type ProposalBondMaximum = ();
	type SpendPeriod = SpendPeriod;
	type SpendOrigin = frame_support::traits::NeverEnsureOrigin<Balance>;
	type Burn = Burn;
	type BurnDestination = ();
	type SpendFunds = ();
	type WeightInfo = ();
	type MaxApprovals = MaxApprovals;
}

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 5 * DAYS;
	pub const CouncilMaxProposals: u32 = 100;
	pub const CouncilMaxMembers: u32 = 100;
}

// TODO: VERY BASIC implementation, more work needed
type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = CouncilMotionDuration;
	type MaxProposals = CouncilMaxProposals;
	type MaxMembers = CouncilMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const TechnicalMotionDuration: BlockNumber = 5 * DAYS;
	pub const TechnicalMaxProposals: u32 = 100;
	pub const TechnicalMaxMembers: u32 = 100;
}

type TechnicalCollective = pallet_collective::Instance2;
impl pallet_collective::Config<TechnicalCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = TechnicalMotionDuration;
	type MaxProposals = TechnicalMaxProposals;
	type MaxMembers = TechnicalMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const LaunchPeriod: BlockNumber = 2 * MINUTES;
	pub const VotingPeriod: BlockNumber = 2 * MINUTES;
	pub const FastTrackVotingPeriod: BlockNumber = 3 * 24 * 60 * MINUTES;
	pub const MinimumDeposit: Balance = 2 * MILLI_PLMC;
	pub const EnactmentPeriod: BlockNumber = MINUTES;
	pub const CooloffPeriod: BlockNumber = 2 * MINUTES;
	pub const MaxProposals: u32 = 100;
	pub const PreimageMaxSize: u32 = 4096 * 1024;
	pub const PreimageBaseDeposit: Balance = PLMC;
}

impl pallet_democracy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type EnactmentPeriod = EnactmentPeriod;
	type LaunchPeriod = LaunchPeriod;
	type VotingPeriod = VotingPeriod;
	type VoteLockingPeriod = EnactmentPeriod; // Same as EnactmentPeriod
	type MinimumDeposit = MinimumDeposit;
	/// A straight majority of the council can decide what their next motion is.
	type ExternalOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>;
	/// A super-majority can have the next scheduled referendum be a straight majority-carries vote.
	type ExternalMajorityOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 4>;
	/// A unanimous council can have the next scheduled referendum be a straight default-carries
	/// (NTB) vote.
	type ExternalDefaultOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 1>;
	/// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
	/// be tabled immediately and with a shorter voting/enactment period.
	type FastTrackOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 2, 3>;
	type InstantOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>;
	type InstantAllowed = frame_support::traits::ConstBool<true>;
	type FastTrackVotingPeriod = FastTrackVotingPeriod;
	// To cancel a proposal which has been passed, 2/3 of the council must agree to it.
	type CancellationOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>;
	// To cancel a proposal before it has been passed, the technical committee must be unanimous or
	// Root must agree.
	type CancelProposalOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>,
	>;
	type BlacklistOrigin = EnsureRoot<AccountId>;
	// Any single technical committee member may veto a coming council proposal, however they can
	// only do it once and it lasts only for the cool-off period.
	type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
	type CooloffPeriod = CooloffPeriod;
	type Slash = ();
	type Scheduler = Scheduler;
	type PalletsOrigin = OriginCaller;
	type MaxVotes = ConstU32<100>;
	type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
	type MaxProposals = MaxProposals;
	type Preimages = ();
	type MaxDeposits = ();
	type MaxBlacklisted = ();
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * RuntimeBlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
}

impl pallet_scheduler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type WeightInfo = ();
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type Preimages = ();
}

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = ();
}

pub const fn deposit(items: u32, bytes: u32) -> Balance {
	items as Balance * 15 * MICRO_PLMC + (bytes as Balance) * 6 * MICRO_PLMC
}

pub const fn free_deposit() -> Balance {
	0 * MICRO_PLMC
}

parameter_types! {
	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = deposit(0, 32);
	pub const MaxSignatories: u16 = 100;
}

impl pallet_multisig::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = ();
}

impl pallet_preimage::Config for Runtime {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type BaseDeposit = PreimageBaseDeposit;
	type ByteDeposit = ();
}

parameter_types! {
	pub const AssetDeposit: Balance = PLMC; // 1 UNIT deposit to create asset
	pub const ApprovalDeposit: Balance = EXISTENTIAL_DEPOSIT;
	pub const AssetAccountDeposit: Balance = deposit(1, 16);
	pub const AssetsStringLimit: u32 = 50;
	/// Key = 32 bytes, Value = 36 bytes (32+1+1+1+1)
	// https://github.com/paritytech/substrate/blob/069917b/frame/assets/src/lib.rs#L257L271
	pub const MetadataDepositBase: Balance = free_deposit();
	pub const MetadataDepositPerByte: Balance = free_deposit();
}

impl pallet_assets::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = AssetId;
	type Currency = Balances;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = AssetsStringLimit;
	type Freezer = ();
	type Extra = ();
	type WeightInfo = ();
	type AssetAccountDeposit = AssetAccountDeposit;
}

impl pallet_asset_registry::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ReserveAssetModifierOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type Assets = Assets;
	type WeightInfo = ();
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		// System support stuff.
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>} = 0,
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 2,
		Sudo: pallet_sudo = 4,
		Utility: pallet_utility::{Pallet, Call, Event} = 5,
		Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>} = 6,

		// Monetary stuff.
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 10,
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Event<T>} = 11,
		AssetTxPayment: pallet_asset_tx_payment::{Pallet, Storage, Event<T>} = 12,
		Assets: pallet_assets::{Pallet, Call, Storage, Event<T>} = 13,
		AssetRegistry: pallet_asset_registry::{Pallet, Call, Storage, Event<T>} = 14,

		// Consensus support.
		// The following order MUST NOT be changed: Aura -> Session -> Staking -> Authorship -> AuraExt
		// Dependencies: AuraExt on Aura, Authorship and Session on ParachainStaking
		Aura: pallet_aura = 23,
		Session: pallet_session = 22,
		ParachainStaking: parachain_staking = 21,
		Authorship: pallet_authorship::{Pallet, Call, Storage} = 20,
		AuraExt: cumulus_pallet_aura_ext = 24,

		// Governance
		Treasury: pallet_treasury = 40,
		Democracy: pallet_democracy = 41,
		Council: pallet_collective::<Instance1> = 42,
		TechnicalCommittee: pallet_collective::<Instance2> = 43,
		Preimage: pallet_preimage::{Pallet, Call, Storage, Event<T>} = 44,

		// Utilities
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>} = 50,
		Random: pallet_randomness_collective_flip = 51,

		// Polimec Core
		PolimecFunding: pallet_funding::{Pallet, Call, Storage, Event<T>}  = 61,
		Credentials: pallet_credentials = 63,

		// Among others: Send and receive DMP and XCMP messages.
		ParachainSystem: cumulus_pallet_parachain_system = 80,
		ParachainInfo: parachain_info::{Pallet, Storage, Config} = 81,
		// Wrap and unwrap XCMP messages to send and receive them. Queue them for later processing.
		XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>} = 82,
		// Build XCM scripts.
		PolkadotXcm: pallet_xcm::{Pallet, Call, Event<T>, Origin, Config} = 83,
		// Does nothing cool, just provides an origin.
		CumulusXcm: cumulus_pallet_xcm::{Pallet, Event<T>, Origin} = 84,
		// Queue and pass DMP messages on to be executed.
		DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>} = 85,
	}
);

#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	define_benchmarks!(
		[frame_system, SystemBench::<Runtime>]
		[pallet_balances, Balances]
		// [pallet_session, SessionBench::<Runtime>]
		[pallet_timestamp, Timestamp]
		[cumulus_pallet_xcmp_queue, XcmpQueue]
		[pallet_funding, PolimecFunding]
	);
}

impl_runtime_apis! {
	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities().into_inner()
		}
	}

	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
		fn account_nonce(account: AccountId) -> Index {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
	}

	impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
		fn collect_collation_info(header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
			ParachainSystem::collect_collation_info(header)
		}
	}


	impl pallet_credentials_runtime_api::CredentialsApi<Block, AccountId> for Runtime {
		fn is_in(role: MemberRole, who: AccountId) -> bool {
			Credentials::is_in(&role, &who)
		}
		fn get_members_of(role: MemberRole) -> Vec<AccountId> {
			Credentials::get_members_of(&role)
		}
		fn get_roles_of(who: AccountId) -> Vec<MemberRole> {
			Credentials::get_roles_of(&who)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade() -> (Weight, Weight) {
			log::info!("try-runtime::on_runtime_upgrade statemine.");
			let weight = Executive::try_runtime_upgrade().unwrap();
			(weight, RuntimeBlockWeights::get().max_block)
		}

		fn execute_block(block: Block, state_root_check: bool, select: frame_try_runtime::TryStateSelect) -> Weight {
			log::info!(
				target: "runtime::statemine", "try-runtime: executing block #{} ({:?}) / root checks: {:?} / sanity-checks: {:?}",
				block.header.number,
				block.header.hash(),
				state_root_check,
				select,
			);
			Executive::try_execute_block(block, state_root_check, select).expect("try_execute_block failed")
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {

			use frame_benchmarking::{Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;
			use frame_system_benchmarking::Pallet as SystemBench;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();
			return (list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{Benchmarking, BenchmarkBatch, TrackedStorageKey};

			use frame_system_benchmarking::Pallet as SystemBench;
			impl frame_system_benchmarking::Config for Runtime {}

			let whitelist: Vec<TrackedStorageKey> = vec![
				// Block Number
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").to_vec().into(),
				// Total Issuance
				hex_literal::hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80").to_vec().into(),
				// Execution Phase
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").to_vec().into(),
				// Event Count
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").to_vec().into(),
				// System Events
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").to_vec().into(),
				//TODO: use from relay_well_known_keys::ACTIVE_CONFIG
				hex_literal::hex!("06de3d8a54d27e44a9d5ce189618f22db4b49d95320d9021994c850f25b8e385").to_vec().into(),
			];

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);

			Ok(batches)
		}
	}
}

struct CheckInherents;

impl cumulus_pallet_parachain_system::CheckInherents<Block> for CheckInherents {
	fn check_inherents(
		block: &Block,
		relay_state_proof: &cumulus_pallet_parachain_system::RelayChainStateProof,
	) -> sp_inherents::CheckInherentsResult {
		let relay_chain_slot = relay_state_proof
			.read_slot()
			.expect("Could not read the relay chain slot from the proof");

		let inherent_data =
			cumulus_primitives_timestamp::InherentDataProvider::from_relay_chain_slot_and_duration(
				relay_chain_slot,
				sp_std::time::Duration::from_secs(6),
			)
			.create_inherent_data()
			.expect("Could not create the timestamp inherent data");

		inherent_data.check_extrinsics(block)
	}
}

cumulus_pallet_parachain_system::register_validate_block! {
	Runtime = Runtime,
	BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
	CheckInherents = CheckInherents,
}
