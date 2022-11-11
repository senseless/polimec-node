use crate::{mock::*, Error, Project};
use frame_support::assert_ok;

pub fn last_event() -> Event {
	frame_system::Pallet::<Test>::events().pop().expect("Event expected").event
}

// TODO: Adapt the run_to_block function to Polimec

// pub fn run_to_block(n: BlockNumber) {
// 	while System::block_number() < n {
// 		Auctions::on_finalize(System::block_number());
// 		Balances::on_finalize(System::block_number());
// 		System::on_finalize(System::block_number());
// 		System::set_block_number(System::block_number() + 1);
// 		System::on_initialize(System::block_number());
// 		Balances::on_initialize(System::block_number());
// 		Auctions::on_initialize(System::block_number());
// 	}
// }

const ALICE: AccountId = 1;
const BOB: AccountId = 2;
const CHARLIE: AccountId = 3;

mod creation_round {
	use super::*;
	use crate::{ParticipantsSize, TicketSize};
	use frame_support::assert_noop;

	#[test]
	fn create_works() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};
			assert_ok!(FundingModule::create(Origin::signed(ALICE), project));
			assert_eq!(
				last_event(),
				Event::FundingModule(crate::Event::Created { project_id: 0, issuer: ALICE })
			);
		})
	}

	#[test]
	fn only_issuer_can_create() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};
			assert_noop!(
				FundingModule::create(Origin::signed(BOB), project),
				Error::<Test>::NotAuthorized
			);
		})
	}

	#[test]
	fn project_id_autoincremenet_works() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};
			assert_ok!(FundingModule::create(Origin::signed(ALICE), project.clone()));
			assert_eq!(
				last_event(),
				Event::FundingModule(crate::Event::Created { project_id: 0, issuer: ALICE })
			);
			assert_ok!(FundingModule::create(Origin::signed(ALICE), project));
			assert_eq!(
				last_event(),
				Event::FundingModule(crate::Event::Created { project_id: 1, issuer: ALICE })
			);
		})
	}

	#[test]
	fn price_too_low() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 0,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_noop!(
				FundingModule::create(Origin::signed(ALICE), project),
				Error::<Test>::PriceTooLow
			);
		})
	}

	#[test]
	fn participants_size_error() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: None, maximum: None },
				..Default::default()
			};

			assert_noop!(
				FundingModule::create(Origin::signed(ALICE), project),
				Error::<Test>::ParticipantsSizeError
			);
		})
	}

	#[test]
	fn ticket_size_error() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: None, maximum: None },
				participants_size: ParticipantsSize { minimum: Some(1), maximum: None },
				..Default::default()
			};

			assert_noop!(
				FundingModule::create(Origin::signed(ALICE), project),
				Error::<Test>::TicketSizeError
			);
		})
	}

	#[test]
	#[ignore = "ATM only the first error will be thrown"]
	fn multiple_field_error() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 0,
				ticket_size: TicketSize { minimum: None, maximum: None },
				participants_size: ParticipantsSize { minimum: None, maximum: None },
				..Default::default()
			};

			assert_noop!(
				FundingModule::create(Origin::signed(ALICE), project),
				Error::<Test>::TicketSizeError
			);
		})
	}
}

mod evaluation_round {
	use super::*;
	use crate::{ParticipantsSize, ProjectStatus, TicketSize};
	use frame_support::{assert_noop, traits::OnInitialize};

	#[test]
	fn start_evaluation_works() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_ok!(FundingModule::create(Origin::signed(ALICE), project));
			let project_info = FundingModule::project_info(0, ALICE);
			assert!(project_info.project_status == ProjectStatus::Application);
			assert_ok!(FundingModule::start_evaluation(Origin::signed(ALICE), 0));
			let project_info = FundingModule::project_info(0, ALICE);
			assert!(project_info.project_status == ProjectStatus::EvaluationRound);
		})
	}

	#[test]
	fn evaluation_stops_after_28_days() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_ok!(FundingModule::create(Origin::signed(ALICE), project));
			let ed = FundingModule::project_info(0, ALICE);
			assert!(ed.project_status == ProjectStatus::Application);
			assert_ok!(FundingModule::start_evaluation(Origin::signed(ALICE), 0));
			let ed = FundingModule::project_info(0, ALICE);
			assert!(ed.project_status == ProjectStatus::EvaluationRound);
			let block_number = System::block_number();
			System::set_block_number(block_number + 100);
			FundingModule::on_initialize(System::block_number());
			let ed = FundingModule::project_info(0, ALICE);
			assert!(ed.project_status == ProjectStatus::EvaluationEnded);
		})
	}

	#[test]
	fn basic_bond_works() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_ok!(FundingModule::create(Origin::signed(ALICE), project));
			assert_noop!(
				FundingModule::bond(Origin::signed(BOB), 0, 128),
				Error::<Test>::EvaluationNotStarted
			);
			assert_ok!(FundingModule::start_evaluation(Origin::signed(ALICE), 0));
			assert_ok!(FundingModule::bond(Origin::signed(BOB), 0, 128));
		})
	}

	#[test]
	fn multiple_bond_works() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_ok!(FundingModule::create(Origin::signed(ALICE), project));
			assert_noop!(
				FundingModule::bond(Origin::signed(BOB), 0, 128),
				Error::<Test>::EvaluationNotStarted
			);
			assert_ok!(FundingModule::start_evaluation(Origin::signed(ALICE), 0));

			assert_ok!(FundingModule::bond(Origin::signed(BOB), 0, 128));
			let evaluation_metadata = FundingModule::evaluations(0, ALICE);
			assert_eq!(evaluation_metadata.amount_bonded, 128);

			assert_ok!(FundingModule::bond(Origin::signed(CHARLIE), 0, 128));
			let evaluation_metadata = FundingModule::evaluations(0, ALICE);
			assert_eq!(evaluation_metadata.amount_bonded, 256);

			let bonds = FundingModule::bonds(0, BOB);
			assert_eq!(bonds.unwrap(), 128);

			let bonds = FundingModule::bonds(0, CHARLIE);
			assert_eq!(bonds.unwrap(), 128);
		})
	}

	#[test]
	fn cannot_bond() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};
			assert_ok!(FundingModule::create(Origin::signed(ALICE), project));
			assert_ok!(FundingModule::start_evaluation(Origin::signed(ALICE), 0));

			assert_noop!(
				FundingModule::bond(Origin::signed(BOB), 0, 1024),
				Error::<Test>::InsufficientBalance
			);
		})
	}
}

mod auction_round {
	use super::*;
	use crate::{ParticipantsSize, TicketSize};
	use frame_support::{assert_noop, traits::OnInitialize};

	#[test]
	fn start_auction_works() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_ok!(FundingModule::create(Origin::signed(ALICE), project));
			assert_ok!(FundingModule::start_evaluation(Origin::signed(ALICE), 0));
			let block_number = System::block_number();
			System::set_block_number(block_number + 100);
			FundingModule::on_initialize(System::block_number());
			assert_ok!(FundingModule::start_auction(Origin::signed(ALICE), 0));
		})
	}

	#[test]
	fn cannot_start_auction_before_evaluation() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_ok!(FundingModule::create(Origin::signed(ALICE), project));
			assert_noop!(
				FundingModule::start_auction(Origin::signed(ALICE), 0),
				Error::<Test>::EvaluationNotStarted
			);
		})
	}

	#[test]
	fn bid_works() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_ok!(FundingModule::create(Origin::signed(ALICE), project));
			assert_ok!(FundingModule::start_evaluation(Origin::signed(ALICE), 0));
			let block_number = System::block_number();
			System::set_block_number(block_number + 100);
			FundingModule::on_initialize(System::block_number());
			assert_ok!(FundingModule::start_auction(Origin::signed(ALICE), 0));
			assert_ok!(FundingModule::bid(Origin::signed(BOB), 0, 1, 100));
			let bids = FundingModule::auctions_info(0, BOB);
			assert!(bids.amount == 100);
			assert!(bids.market_cap == 1);
			assert!(bids.when == block_number + 100);
		})
	}

	#[test]
	fn cannot_bid_before_auction_round() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_ok!(FundingModule::create(Origin::signed(ALICE), project));
			assert_ok!(FundingModule::start_evaluation(Origin::signed(ALICE), 0));
			assert_noop!(
				FundingModule::bid(Origin::signed(BOB), 0, 1, 100),
				Error::<Test>::AuctionNotStarted
			);
		})
	}

	#[test]
	fn contribute_does_not_work() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_ok!(FundingModule::create(Origin::signed(ALICE), project));
			assert_ok!(FundingModule::start_evaluation(Origin::signed(ALICE), 0));
			let block_number = System::block_number();
			System::set_block_number(block_number + 100);
			FundingModule::on_initialize(System::block_number());
			assert_ok!(FundingModule::start_auction(Origin::signed(ALICE), 0));
			assert_noop!(
				FundingModule::contribute(Origin::signed(BOB), 0, 100),
				Error::<Test>::AuctionNotStarted
			);
		})
	}
}

mod community_round {
	#[test]
	fn contribute_works() {}
}

mod flow {
	use super::*;
	use crate::{AuctionPhase, ParticipantsSize, ProjectStatus, TicketSize};
	use frame_support::{
		pallet_prelude::Weight,
		traits::{OnIdle, OnInitialize},
	};

	#[test]
	fn it_works() {
		new_test_ext().execute_with(|| {
			// Create a new project
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};
			assert_ok!(FundingModule::create(Origin::signed(ALICE), project));
			let project_info = FundingModule::project_info(0, ALICE);
			assert!(project_info.project_status == ProjectStatus::Application);

			// Start the Evaluation Round
			assert_ok!(FundingModule::start_evaluation(Origin::signed(ALICE), 0));
			let active_projects = FundingModule::projects_active();
			assert!(active_projects.len() == 1);
			let project_info = FundingModule::project_info(0, ALICE);
			assert!(project_info.project_status == ProjectStatus::EvaluationRound);
			assert_ok!(FundingModule::bond(Origin::signed(BOB), 0, 128));

			// Evaluation Round ends automatically
			let block_number = System::block_number();
			System::set_block_number(block_number + 28);
			FundingModule::on_initialize(System::block_number());
			let project_info = FundingModule::project_info(0, ALICE);
			assert!(project_info.project_status == ProjectStatus::EvaluationEnded);

			// Start the Funding Round: 1) English Auction Round
			assert_ok!(FundingModule::start_auction(Origin::signed(ALICE), 0));
			let project_info = FundingModule::project_info(0, ALICE);
			assert!(
				project_info.project_status == ProjectStatus::AuctionRound(AuctionPhase::English)
			);
			assert_ok!(FundingModule::bid(Origin::signed(BOB), 0, 1, 100));

			// Second phase of Funding Round: 2) Candle Auction Round
			let block_number = System::block_number();
			System::set_block_number(block_number + 10);
			FundingModule::on_initialize(System::block_number());
			let project_info = FundingModule::project_info(0, ALICE);
			assert!(
				project_info.project_status == ProjectStatus::AuctionRound(AuctionPhase::Candle)
			);
			assert_ok!(FundingModule::bid(Origin::signed(CHARLIE), 0, 2, 200));

			// Third phase of Funding Round: 3) Community Round
			let block_number = System::block_number();
			System::set_block_number(block_number + 5);
			FundingModule::on_initialize(System::block_number());
			let project_info = FundingModule::project_info(0, ALICE);
			assert!(project_info.project_status == ProjectStatus::CommunityRound);
			assert_ok!(FundingModule::contribute(Origin::signed(BOB), 0, 100));

			// Funding Round ends
			let block_number = System::block_number();
			System::set_block_number(block_number + 10);
			FundingModule::on_initialize(System::block_number());
			let project_info = FundingModule::project_info(0, ALICE);
			assert!(project_info.project_status == ProjectStatus::FundingEnded);
			System::set_block_number(block_number + 10);
			FundingModule::on_initialize(System::block_number());
			FundingModule::on_idle(System::block_number(), Weight::from_ref_time(10000000));
			let project_info = FundingModule::project_info(0, ALICE);
			assert!(project_info.project_status == ProjectStatus::ReadyToLaunch);
			// Project is no longer "active"
			let active_projects = FundingModule::projects_active();
			assert!(active_projects.len() == 0);
		})
	}

	#[test]
	#[ignore = "Final Price calculation not ready yet"]
	fn check_final_price() {
		new_test_ext().execute_with(|| {
			// Create a new project
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				total_allocation_size: 100000,
				..Default::default()
			};
			assert_ok!(FundingModule::create(Origin::signed(ALICE), project));
			let project_info = FundingModule::project_info(0, ALICE);
			assert!(project_info.project_status == ProjectStatus::Application);

			// Start the Evaluation Round
			assert_ok!(FundingModule::start_evaluation(Origin::signed(ALICE), 0));
			let active_projects = FundingModule::projects_active();
			assert!(active_projects.len() == 1);
			let project_info = FundingModule::project_info(0, ALICE);
			assert!(project_info.project_status == ProjectStatus::EvaluationRound);
			assert_ok!(FundingModule::bond(Origin::signed(BOB), 0, 128));

			// Evaluation Round ends automatically
			let block_number = System::block_number();
			System::set_block_number(block_number + 28);
			FundingModule::on_initialize(System::block_number());
			let project_info = FundingModule::project_info(0, ALICE);
			assert!(project_info.project_status == ProjectStatus::EvaluationEnded);

			// Start the Funding Round: 1) English Auction Round
			assert_ok!(FundingModule::start_auction(Origin::signed(ALICE), 0));
			let project_info = FundingModule::project_info(0, ALICE);
			assert!(
				project_info.project_status == ProjectStatus::AuctionRound(AuctionPhase::English)
			);
			assert_ok!(FundingModule::bid(Origin::signed(BOB), 0, 19, 17));

			// Second phase of Funding Round: 2) Candle Auction Round
			let block_number = System::block_number();
			System::set_block_number(block_number + 10);
			FundingModule::on_initialize(System::block_number());
			let project_info = FundingModule::project_info(0, ALICE);
			assert!(
				project_info.project_status == ProjectStatus::AuctionRound(AuctionPhase::Candle)
			);
			assert_ok!(FundingModule::bid(Origin::signed(CHARLIE), 0, 74, 2));
			assert_ok!(FundingModule::bid(Origin::signed(3), 0, 16, 35));
			assert_ok!(FundingModule::bid(Origin::signed(4), 0, 15, 20));
			assert_ok!(FundingModule::bid(Origin::signed(4), 0, 12, 55));

			let auction_info_3 = FundingModule::auctions_info(0, 3);
			println!("Bid Info {auction_info_3:?}");

			let block_number = System::block_number();
			System::set_block_number(block_number + 10);
			FundingModule::on_initialize(System::block_number());
			let project_info = FundingModule::project_info(0, ALICE);
			println!("Final Price {:?}", project_info.final_price);
		})
	}
}

mod final_price {
	use crate::BidInfo;
	use sp_std::cmp::Reverse;

	use super::*;
	#[test]

	fn final_price_check() {
		new_test_ext().execute_with(|| {
			const UNIT: u128 = 10_000_000_000;
			let total_allocation_size = 1000 * UNIT;
			let mut bids: Vec<BidInfo<u128, u64>> = vec![
				BidInfo { amount: 17 * UNIT, market_cap: 19 * UNIT, when: 1 },
				BidInfo { amount: UNIT, market_cap: 74 * UNIT, when: 2 },
				BidInfo { amount: 8 * UNIT, market_cap: 10 * UNIT, when: 3 },
				BidInfo { amount: 55 * UNIT, market_cap: 12 * UNIT, when: 4 },
				BidInfo { amount: 20 * UNIT, market_cap: 15 * UNIT, when: 5 },
				BidInfo { amount: 3 * UNIT, market_cap: 16 * UNIT, when: 6 },
				BidInfo { amount: 50 * UNIT, market_cap: 12 * UNIT, when: 7 },
				BidInfo { amount: 60 * UNIT, market_cap: 7 * UNIT, when: 8 },
			];
			bids.sort_by_key(|bid| Reverse(bid.market_cap));
			println!("{:#?}", bids);
			let value = FundingModule::final_price_logic(bids, total_allocation_size);
			match value {
				Ok(num) => println!("{}", num),
				Err(_) => todo!(),
			}
		})
	}
}

mod random {
	use super::*;
	#[test]
	#[ignore = "Not implemented yet"]
	fn random_is_really_random() {
		new_test_ext().execute_with(|| {
			let random = FundingModule::select_random_block();
			println!("Random {:?}", random);
			let block_number = System::block_number();
			System::set_block_number(block_number + 1000);
			let random = FundingModule::select_random_block();
			println!("Random {:?}", random);
			let block_number = System::block_number();
			System::set_block_number(block_number + 1000);
			let random = FundingModule::select_random_block();
			println!("Random {:?}", random);
			let block_number = System::block_number();
			System::set_block_number(block_number + 1000);
			let random = FundingModule::select_random_block();
			println!("Random {:?}", random);
		})
	}
}