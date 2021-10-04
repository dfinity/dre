use super::cli_types::*;

pub fn merge_opts_into_cfg(opts: &Opts, cfg: &OperatorConfig) -> OperatorConfig {
    let mut temp_opconfig = cfg.clone();
    match &opts.backend_url {
        Some(v) => temp_opconfig.backend_url = Some(v.clone()),
        None => (),
    }
    match &opts.nns_url {
        Some(v) => temp_opconfig.nns_url = Some(v.clone()),
        None => (),
    }
    match &opts.hsm_pin {
        Some(v) => temp_opconfig.hsm_pin = Some(v.clone()),
        None => (),
    }
    match &opts.hsm_slot {
        Some(v) => temp_opconfig.hsm_slot = Some(v.clone()),
        None => (),
    }
    match &opts.hsm_key_id {
        Some(v) => temp_opconfig.hsm_key_id = Some(v.clone()),
        None => (),
    }
    match &opts.neuron_index {
        Some(v) => temp_opconfig.neuron_index = Some(v.clone()),
        None => (),
    }
    match &opts.ic_admin_cmd {
        Some(v) => temp_opconfig.ic_admin_cmd = Some(v.clone()),
        None => (),
    }
    match &opts.proposal_url {
        Some(v) => temp_opconfig.proposal_url = Some(v.clone()),
        None => (),
    }
    temp_opconfig
}
