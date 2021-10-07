use crate::utils;

#[test]
fn proposal_parse_executed() {
    let proposal_text = r#"(
  opt record {
    id = opt record { id = 21_415 };
    status = 4;
    topic = 7;
    failure_reason = null;
    ballots = vec {};
    proposal_timestamp_seconds = 1_632_193_070;
    reward_event_round = 136;
    failed_timestamp_seconds = 0;
    reject_cost_e8s = 100_000_000;
    latest_tally = opt record {
      no = 34_939_792_605;
      yes = 31_326_256_913_663_118;
      total = 31_327_066_556_883_244;
      timestamp_seconds = 1_632_210_267;
    };
    reward_status = 3;
    decided_timestamp_seconds = 1_632_210_267;
    proposal = opt record {
      url = "https://github.com/ic-association/nns-proposals/blob/main/proposals/subnet_management/20210921T0256Z.md";
      action = opt variant {
        ExecuteNnsFunction = record {
          nns_function = 13;
          payload = blob "DIDL\02l\01\bb\f8\fd\ed\0f\01mh\01\00\01\01\1d0\95=K\feV\ef\aci\cb\bc\5c\ff5{\f0\da[\f7\fah\c0*y\8c#\0d\c7\02";
        }
      };
      summary = "Remove node(s) from subnet(s)";
    };
    proposer = opt record { id = 68 };
    executed_timestamp_seconds = 1_632_210_267;
  },
)"#;

    let result = utils::proposal_text_parse(&proposal_text.to_string());
    let proposal = result.unwrap();

    assert_eq!(
        proposal,
        utils::ProposalStatus {
            id: 21_415,
            summary: "Remove node(s) from subnet(s)".to_string(),
            timestamp_seconds: 1_632_193_070,
            executed_timestamp_seconds: 1_632_210_267,
            failed_timestamp_seconds: 0,
            failure_reason: "null".to_string(),
        }
    )
}

#[test]
fn proposal_parse_failed() {
    let proposal_text = r#"(
  opt record {
    id = opt record { id = 21_509 };
    status = 5;
    topic = 7;
    failure_reason = opt record {
      error_message = "Error executing ExecuteNnsFunction proposal. Rejection message: IC0503: Canister rwlgt-iiaaa-aaaaa-aaaaa-cai trapped explicitly: Panicked at 'Repeated nodes in subnet eq6en-6jqla-fbu5s-daskr-h6hx2-376n5-iqabl-qgrng-gfqmv-n3yjr-mqe', registry/canister/src/invariants.rs:562:17";
      error_type = 12;
    };
    ballots = vec {};
    proposal_timestamp_seconds = 1_632_237_821;
    reward_event_round = 136;
    failed_timestamp_seconds = 1_632_290_616;
    reject_cost_e8s = 100_000_000;
    latest_tally = opt record {
      no = 66_128_592_222;
      yes = 31_324_632_574_169_549;
      total = 31_325_466_591_461_761;
      timestamp_seconds = 1_632_290_616;
    };
    reward_status = 3;
    decided_timestamp_seconds = 1_632_290_616;
    proposal = opt record {
      url = "https://github.com/ic-association/nns-proposals/blob/main/proposals/subnet_management/20210921T1521Z.md";
      action = opt variant {
        ExecuteNnsFunction = record {
          nns_function = 2;
          payload = blob "DIDL\02l\02\bd\86\9d\8b\04h\bb\f8\fd\ed\0f\01mh\01\00\01\1d0X\0a\1avC\04\95\13\f8\f7\d6\ff\e6\f5\10\00W\03E\a61`\ca\b7xLY\02\01\01\1d\0a|\fb\a7\c8\f2\09\ee\93h.\5c\b2y\e3)\aa\d9\1e\5c\a6:\86\93\cb\a5P\0e\02";
        }
      };
      summary = "Add node(s) to subnet 10";
    };
    proposer = opt record { id = 39 };
    executed_timestamp_seconds = 0;
  },
)"#;

    let result = utils::proposal_text_parse(&proposal_text.to_string());
    assert!(result.is_ok());
    let proposal = result.unwrap();

    assert_eq!(
        proposal,
        utils::ProposalStatus {
            id: 21_509,
            summary: "Add node(s) to subnet 10".to_string(),
            timestamp_seconds: 1_632_237_821,
            executed_timestamp_seconds: 0,
            failed_timestamp_seconds: 1_632_290_616,
            failure_reason: r#"opt record {
      error_message = "Error executing ExecuteNnsFunction proposal. Rejection message: IC0503: Canister rwlgt-iiaaa-aaaaa-aaaaa-cai trapped explicitly: Panicked at 'Repeated nodes in subnet eq6en-6jqla-fbu5s-daskr-h6hx2-376n5-iqabl-qgrng-gfqmv-n3yjr-mqe', registry/canister/src/invariants.rs:562:17";
      error_type = 12;
    }"#.to_string(),
        }
    )
}
