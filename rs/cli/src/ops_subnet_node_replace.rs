use crate::ic_admin;
use decentralization::SubnetChangeResponse;

#[cfg(test)]
mod tests;

pub fn replace_proposal_options(change: &SubnetChangeResponse) -> anyhow::Result<ic_admin::ProposeOptions> {
    let subnet_id = change
        .subnet_id
        .ok_or_else(|| anyhow::anyhow!("subnet_id is required"))?
        .to_string();

    let replace_target = if change.added.len() > 1 || change.removed.len() > 1 {
        "nodes"
    } else {
        "a node"
    };
    let subnet_id_short = subnet_id.split('-').next().unwrap();

    Ok(ic_admin::ProposeOptions {
        title: format!("Replace {replace_target} in subnet {subnet_id_short}",).into(),
        summary: format!("# Replace {replace_target} in subnet {subnet_id_short}",).into(),
        motivation: change.motivation.clone(),
    })
}
