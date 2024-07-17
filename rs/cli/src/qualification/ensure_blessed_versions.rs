use async_trait::async_trait;

use crate::qualification::Step;

use super::{
    print_table,
    tabular_util::{ColumnAlignment, Table},
    QualificationContext,
};

pub struct EnsureBlessedRevisions {}

#[async_trait]
impl Step for EnsureBlessedRevisions {
    fn help(&self) -> &'static str {
        "This step runs the check to determine if all versions are blessed"
    }

    fn name(&self) -> &'static str {
        "1a_ensure_blessed_revision"
    }

    async fn execute(&self, ctx: &QualificationContext) -> anyhow::Result<()> {
        Ok(())
    }

    async fn print_status(&self, ctx: &QualificationContext) -> anyhow::Result<()> {
        let table = Table::new()
            .with_columns(&[("Subnet Id", ColumnAlignment::Middle), ("Version", ColumnAlignment::Middle)])
            .with_rows(vec![vec!["id1".to_string(), "v1".to_string()]])
            .to_table();

        print_table(table);

        Ok(())
    }
}
