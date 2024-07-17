use async_trait::async_trait;
use chrono::Utc;
use ensure_blessed_versions::EnsureBlessedRevisions;
use itertools::Itertools;
use tabular_util::{ColumnAlignment, Table};

use crate::ctx::DreContext;

mod ensure_blessed_versions;
mod tabular_util;

pub struct QualificationExecutor {
    steps: Vec<Box<dyn Step>>,
}

pub struct QualificationContext {
    dre_ctx: DreContext,
}

impl QualificationContext {
    pub fn new(dre_ctx: DreContext) -> Self {
        Self { dre_ctx }
    }

    pub fn dre_ctx(&self) -> &DreContext {
        &self.dre_ctx
    }
}

impl QualificationExecutor {
    pub fn with_steps() -> Self {
        Self {
            steps: vec![Box::new(EnsureBlessedRevisions {})],
        }
    }

    pub fn list(&self) {
        let table = Table::new()
            .with_columns(&[("Name", ColumnAlignment::Middle), ("Help", ColumnAlignment::Left)])
            .with_rows(self.steps.iter().map(|s| vec![s.name().to_string(), s.help().to_string()]).collect_vec())
            .to_table();

        println!("{}", table)
    }

    pub async fn execute(&self, ctx: QualificationContext) -> anyhow::Result<()> {
        print_text(format!("Starting execution of {} steps:", self.steps.len()));
        for (i, step) in self.steps.iter().enumerate() {
            print_text(format!("Executing step {}: `{}`", i, step.name()));

            step.execute(&ctx).await?;

            print_text(format!("Executed step {}: `{}`", i, step.name()));

            step.print_status(&ctx).await?
        }

        Ok(())
    }
}

#[async_trait]
pub trait Step {
    fn help(&self) -> &'static str;

    fn name(&self) -> &'static str;

    async fn execute(&self, ctx: &QualificationContext) -> anyhow::Result<()>;

    async fn print_status(&self, ctx: &QualificationContext) -> anyhow::Result<()>;
}

pub fn print_text(message: String) {
    print_with_time(message, false)
}

pub fn print_table(table: tabular::Table) {
    print_with_time(format!("{}", table), true)
}

fn print_with_time(message: String, add_new_line: bool) {
    let current_time = Utc::now();

    println!(
        "[{}]{}{}",
        current_time,
        match add_new_line {
            true => '\n',
            false => ' ',
        },
        message
    )
}
