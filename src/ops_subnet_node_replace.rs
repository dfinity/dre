use crate::ic_admin;
use decentralization::SubnetChangeResponse;
use ic_base_types::PrincipalId;

#[cfg(test)]
mod tests;

pub fn replace_proposal_options(
    change: &SubnetChangeResponse,
    first_proposal_id: Option<u64>,
) -> anyhow::Result<ic_admin::ProposeOptions> {
    let subnet_id = change
        .subnet_id
        .ok_or_else(|| anyhow::anyhow!("subnet_id is required"))?
        .to_string();

    let concat_principals =
        |principals: &[PrincipalId]| principals.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ");

    let replace_target = if change.added.len() > 1 || change.removed.len() > 1 {
        "nodes"
    } else {
        "a node"
    };
    let subnet_id_short = subnet_id.split('-').next().unwrap();

    Ok(ic_admin::ProposeOptions {
        title: format!("Replace {replace_target} in subnet {subnet_id_short}",).into(),
        summary: format!(
            r#"# Replace {replace_target} in subnet {subnet_id_short}.

- Step 1 ({step_1_details}): Add nodes [{add_nodes}]
- Step 2 ({step_2_details}): Remove nodes [{remove_nodes}]
"#,
            step_1_details = first_proposal_id
                .map(|id| format!("proposal [{id}](https://dashboard.internetcomputer.org/proposal/{id})"))
                .unwrap_or_else(|| "this proposal".to_string()),
            add_nodes = concat_principals(&change.added),
            step_2_details = if first_proposal_id.is_some() {
                "this proposal"
            } else {
                "upcoming proposal"
            },
            remove_nodes = concat_principals(&change.removed),
        )
        .into(),
        motivation: change.motivation.clone(),
    })
}
