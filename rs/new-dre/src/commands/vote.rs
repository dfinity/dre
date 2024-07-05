use std::time::Duration;

use clap::Args;
use humantime::parse_duration;

#[derive(Args, Debug)]
pub struct Vote {
    /// Override default accepted proposers
    /// These are the proposers which proposals will
    /// be automatically voted on
    ///
    /// By default: DRE + automation neuron 80
    #[clap(
        long,
        use_value_delimiter = true,
        value_delimiter = ',',
        value_name = "PROPOSER_ID",
        default_value = "80,39,40,46,58,61,77"
    )]
    pub accepted_neurons: Vec<u64>,

    /// Override default topics to vote on
    /// Use with caution! This is subcommand is intended to be used
    /// only by DRE in processes of rolling out new versions,
    /// everything else should be double checked manually
    ///
    /// By default: SubnetReplicaVersionManagement
    #[clap(long, use_value_delimiter = true, value_delimiter = ',', value_name = "PROPOSER_ID", default_value = "12")]
    pub accepted_topics: Vec<i32>,

    /// Override default sleep time
    #[clap(long, default_value = "60s", value_parser = parse_duration)]
    pub sleep_time: Duration,
}
