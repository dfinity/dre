use clap::Parser;
use ic_base_types::PrincipalId;

use crate::commands::subnet::{Subcommands, Subnet};

fn id(i: u64) -> PrincipalId {
    PrincipalId::new_node_test_id(i)
}

#[test]
fn parse_create_add_nodes_aliases() {
    let args = ["--size", "13", "--add-nodes", &id(1).to_string(), &id(2).to_string(), "--motivation", "m"];
    let s = Subnet::parse_from(["subnet"].into_iter().chain(["create"]).chain(args));
    let Subcommands::Create(c) = s.subcommands else {
        panic!("expected create")
    };
    assert_eq!(c.add_nodes.len(), 2);

    let args = ["--size", "13", "--add", &id(1).to_string(), &id(2).to_string(), "--motivation", "m"];
    let s = Subnet::parse_from(["subnet"].into_iter().chain(["create"]).chain(args));
    let Subcommands::Create(c) = s.subcommands else {
        panic!("expected create")
    };
    assert_eq!(c.add_nodes.len(), 2);
}

#[test]
fn parse_resize_add_nodes_aliases() {
    let args = [
        "--id",
        &PrincipalId::new_subnet_test_id(1).to_string(),
        "--motivation",
        "m",
        "--add-nodes",
        &id(1).to_string(),
        &id(2).to_string(),
    ];
    let s = Subnet::parse_from(["subnet"].into_iter().chain(["resize"]).chain(args));
    let Subcommands::Resize(r) = s.subcommands else {
        panic!("expected resize")
    };
    assert_eq!(r.add_nodes.len(), 2);

    let args = [
        "--id",
        &PrincipalId::new_subnet_test_id(1).to_string(),
        "--motivation",
        "m",
        "--add",
        &id(1).to_string(),
        &id(2).to_string(),
    ];
    let s = Subnet::parse_from(["subnet"].into_iter().chain(["resize"]).chain(args));
    let Subcommands::Resize(r) = s.subcommands else {
        panic!("expected resize")
    };
    assert_eq!(r.add_nodes.len(), 2);
}

#[test]
fn parse_replace_remove_and_count() {
    let args = [
        "--id",
        &PrincipalId::new_subnet_test_id(1).to_string(),
        "--remove-nodes",
        &id(1).to_string(),
        &id(2).to_string(),
        "--motivation",
        "m",
    ];
    let s = Subnet::parse_from(["subnet"].into_iter().chain(["replace"]).chain(args));
    let Subcommands::Replace(r) = s.subcommands else {
        panic!("expected replace")
    };
    assert_eq!(r.nodes.len(), 2);

    let args = [
        "--id",
        &PrincipalId::new_subnet_test_id(1).to_string(),
        "--replace-count",
        "2",
        "--motivation",
        "m",
    ];
    let s = Subnet::parse_from(["subnet"].into_iter().chain(["replace"]).chain(args));
    let Subcommands::Replace(r) = s.subcommands else {
        panic!("expected replace")
    };
    assert_eq!(r.optimize, Some(2));

    // alias
    let args = [
        "--id",
        &PrincipalId::new_subnet_test_id(1).to_string(),
        "--optimize",
        "2",
        "--motivation",
        "m",
    ];
    let s = Subnet::parse_from(["subnet"].into_iter().chain(["replace"]).chain(args));
    let Subcommands::Replace(r) = s.subcommands else {
        panic!("expected replace")
    };
    assert_eq!(r.optimize, Some(2));
}
