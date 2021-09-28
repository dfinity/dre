use super::cli_types::*;
use super::types::*;


pub fn merge_opts_into_cfg(opts: &Opts, mut cfg: OperatorConfig) -> OperatorConfig {
    match &opts.backend_url {
        Some(v) => { cfg.backend_url = Some(v.clone()) },
        None => (),
    }
    match &opts.nns_url {
        Some(v) => { cfg.nns_url = Some(v.clone()) },
        None => (),
    }
    match &opts.hsm_pin {
        Some(v) => { cfg.hsm_pin = Some(v.clone()) },
        None => (),
    }
    match &opts.hsm_slot {
        Some(v) => { cfg.hsm_slot = Some(v.clone()) },
        None => (),
    }
    match &opts.hsm_key_id {
        Some(v) => { cfg.hsm_key_id = Some(v.clone()) },
        None => (),
    }
    match &opts.neuron_index {
        Some(v) => { cfg.neuron_index = Some(v.clone()) },
        None => (),
    }
    match &opts.ic_admin_cmd {
        Some(v) => { cfg.ic_admin_cmd = Some(v.clone()) },
        None => (),
    }
    match &opts.proposal_url {
        Some(v) => { cfg.proposal_url = Some(v.clone()) },
        None => (),
    }
    cfg
}