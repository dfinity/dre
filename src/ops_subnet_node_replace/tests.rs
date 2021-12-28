use crate::ops_subnet_node_replace;

#[test]
fn proposal_generate_summary_1_node() {
    let subnet_id = "tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe".to_string();
    let nodes_to_add = "afx6y-22h67-ct72t-etddn-t2jaz-gfsrz-u3yxw-oocjp-gj3za-de3ot-2ae".to_string();
    let nodes_to_remove = "z3tum-w7bue-lt6ca-qgynf-us6oq-nc3qc-7miiq-34rbp-ekuoa-g6cqr-wqe".to_string();

    let summary = ops_subnet_node_replace::proposal_generate_summary(&subnet_id, &nodes_to_add, &nodes_to_remove, 0);

    assert_eq!(
        summary,
        r#"# Replace a node in subnet tdb26.

- Step 1 (this proposal): Add nodes [afx6y-22h67-ct72t-etddn-t2jaz-gfsrz-u3yxw-oocjp-gj3za-de3ot-2ae]
- Step 2 (upcoming proposal): Remove nodes [z3tum-w7bue-lt6ca-qgynf-us6oq-nc3qc-7miiq-34rbp-ekuoa-g6cqr-wqe]
"#
    );

    let summary = ops_subnet_node_replace::proposal_generate_summary(&subnet_id, &nodes_to_add, &nodes_to_remove, 123);

    assert_eq!(
        summary,
        r#"# Replace a node in subnet tdb26.

- Step 1 (proposal [123](https://dashboard.internetcomputer.org/proposal/123)): Add nodes [afx6y-22h67-ct72t-etddn-t2jaz-gfsrz-u3yxw-oocjp-gj3za-de3ot-2ae]
- Step 2 (this proposal): Remove nodes [z3tum-w7bue-lt6ca-qgynf-us6oq-nc3qc-7miiq-34rbp-ekuoa-g6cqr-wqe]
"#
    );
}

#[test]
fn proposal_generate_summary_2_nodes() {
    let subnet_id = "tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe".to_string();
    let nodes_to_add =
        "afx6y-22h67-ct72t-etddn-t2jaz-gfsrz-u3yxw-oocjp-gj3za-de3ot-2ae,dsthq-itfw5-zkibk-chtl5-u7afl-xvxva-7swke-tvqif-vq3t2-wvp7x-mae".to_string();
    let nodes_to_remove =
        "z3tum-w7bue-lt6ca-qgynf-us6oq-nc3qc-7miiq-34rbp-ekuoa-g6cqr-wqe,ktrkp-ccur6-nvpyb-sokhh-exg7x-pfuds-4jxmw-n2r5m-vj5yt-aqzc4-vae".to_string();

    let summary = ops_subnet_node_replace::proposal_generate_summary(&subnet_id, &nodes_to_add, &nodes_to_remove, 0);

    assert_eq!(
        summary,
        r#"# Replace nodes in subnet tdb26.

- Step 1 (this proposal): Add nodes [afx6y-22h67-ct72t-etddn-t2jaz-gfsrz-u3yxw-oocjp-gj3za-de3ot-2ae, dsthq-itfw5-zkibk-chtl5-u7afl-xvxva-7swke-tvqif-vq3t2-wvp7x-mae]
- Step 2 (upcoming proposal): Remove nodes [z3tum-w7bue-lt6ca-qgynf-us6oq-nc3qc-7miiq-34rbp-ekuoa-g6cqr-wqe, ktrkp-ccur6-nvpyb-sokhh-exg7x-pfuds-4jxmw-n2r5m-vj5yt-aqzc4-vae]
"#
    );

    let summary = ops_subnet_node_replace::proposal_generate_summary(&subnet_id, &nodes_to_add, &nodes_to_remove, 123);

    assert_eq!(
        summary,
        r#"# Replace nodes in subnet tdb26.

- Step 1 (proposal [123](https://dashboard.internetcomputer.org/proposal/123)): Add nodes [afx6y-22h67-ct72t-etddn-t2jaz-gfsrz-u3yxw-oocjp-gj3za-de3ot-2ae, dsthq-itfw5-zkibk-chtl5-u7afl-xvxva-7swke-tvqif-vq3t2-wvp7x-mae]
- Step 2 (this proposal): Remove nodes [z3tum-w7bue-lt6ca-qgynf-us6oq-nc3qc-7miiq-34rbp-ekuoa-g6cqr-wqe, ktrkp-ccur6-nvpyb-sokhh-exg7x-pfuds-4jxmw-n2r5m-vj5yt-aqzc4-vae]
"#
    );
}

#[test]
fn proposal_generate_title_1_node() {
    let subnet_id = "tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe".to_string();
    let nodes_to_add = "afx6y-22h67-ct72t-etddn-t2jaz-gfsrz-u3yxw-oocjp-gj3za-de3ot-2ae".to_string();
    let nodes_to_remove = "z3tum-w7bue-lt6ca-qgynf-us6oq-nc3qc-7miiq-34rbp-ekuoa-g6cqr-wqe".to_string();

    let title = ops_subnet_node_replace::proposal_generate_title(&subnet_id, &nodes_to_add, &nodes_to_remove, 0);

    assert_eq!(
        title,
        "Replace a node in subnet tdb26: add nodes [afx6y] (in this proposal); remove nodes [z3tum] (in an upcoming proposal)"
    );

    let title = ops_subnet_node_replace::proposal_generate_title(&subnet_id, &nodes_to_add, &nodes_to_remove, 123);

    assert_eq!(
        title,
        "Replace a node in subnet tdb26: add nodes [afx6y] (already done); remove nodes [z3tum] (in this proposal)"
    );
}

#[test]
fn proposal_generate_title_2_nodes() {
    let subnet_id = "tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe".to_string();
    let nodes_to_add =
        "afx6y-22h67-ct72t-etddn-t2jaz-gfsrz-u3yxw-oocjp-gj3za-de3ot-2ae,dsthq-itfw5-zkibk-chtl5-u7afl-xvxva-7swke-tvqif-vq3t2-wvp7x-mae".to_string();
    let nodes_to_remove =
        "z3tum-w7bue-lt6ca-qgynf-us6oq-nc3qc-7miiq-34rbp-ekuoa-g6cqr-wqe,ktrkp-ccur6-nvpyb-sokhh-exg7x-pfuds-4jxmw-n2r5m-vj5yt-aqzc4-vae".to_string();

    let title = ops_subnet_node_replace::proposal_generate_title(&subnet_id, &nodes_to_add, &nodes_to_remove, 0);

    assert_eq!(
        title,
        "Replace nodes in subnet tdb26: add nodes [afx6y, dsthq] (in this proposal); remove nodes [z3tum, ktrkp] (in an upcoming proposal)"
    );

    let title = ops_subnet_node_replace::proposal_generate_title(&subnet_id, &nodes_to_add, &nodes_to_remove, 123);

    assert_eq!(
        title,
        "Replace nodes in subnet tdb26: add nodes [afx6y, dsthq] (already done); remove nodes [z3tum, ktrkp] (in this proposal)"
    );
}
