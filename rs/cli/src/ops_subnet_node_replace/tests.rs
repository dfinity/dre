use std::str::FromStr;

use decentralization::SubnetChangeResponse;
use ic_base_types::PrincipalId;

use crate::ops_subnet_node_replace;

#[test]
fn replace_proposal_options_1_node() {
    let change = SubnetChangeResponse {
        subnet_id: PrincipalId::from_str("tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe")
            .unwrap()
            .into(),
        added: vec![PrincipalId::from_str("afx6y-22h67-ct72t-etddn-t2jaz-gfsrz-u3yxw-oocjp-gj3za-de3ot-2ae").unwrap()],
        removed: vec![
            PrincipalId::from_str("z3tum-w7bue-lt6ca-qgynf-us6oq-nc3qc-7miiq-34rbp-ekuoa-g6cqr-wqe").unwrap(),
        ],
        motivation: Some("For testing purposes".to_string()),
        ..Default::default()
    };

    let result = ops_subnet_node_replace::replace_proposal_options(&change).unwrap();

    assert_eq!(result.summary.unwrap(), "# Replace a node in subnet tdb26");
    assert_eq!(result.motivation.unwrap(), "For testing purposes");
}

#[test]
fn replace_proposal_options_2_nodes() {
    let change = SubnetChangeResponse {
        subnet_id: PrincipalId::from_str("tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe")
            .unwrap()
            .into(),
        added: vec![
            PrincipalId::from_str("afx6y-22h67-ct72t-etddn-t2jaz-gfsrz-u3yxw-oocjp-gj3za-de3ot-2ae").unwrap(),
            PrincipalId::from_str("dsthq-itfw5-zkibk-chtl5-u7afl-xvxva-7swke-tvqif-vq3t2-wvp7x-mae").unwrap(),
        ],
        removed: vec![
            PrincipalId::from_str("z3tum-w7bue-lt6ca-qgynf-us6oq-nc3qc-7miiq-34rbp-ekuoa-g6cqr-wqe").unwrap(),
            PrincipalId::from_str("ktrkp-ccur6-nvpyb-sokhh-exg7x-pfuds-4jxmw-n2r5m-vj5yt-aqzc4-vae").unwrap(),
        ],
        motivation: Some("For testing purposes".to_string()),
        ..Default::default()
    };

    let result = ops_subnet_node_replace::replace_proposal_options(&change).unwrap();

    assert_eq!(result.summary.unwrap(), "# Replace nodes in subnet tdb26");
    assert_eq!(result.motivation.unwrap(), "For testing purposes");
}
