use clap::Args;

#[derive(Args, Debug)]
pub struct Heal {
    #[clap(
        short,
        long,
        help = r#"Maximum number of nodes to be replaced per subnet.
Optimization will be performed automatically maximizing the decentralization and
minimizing the number of replaced nodes per subnet"#
    )]
    pub max_replaceable_nodes_per_sub: Option<usize>,
}
