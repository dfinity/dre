use std::collections::BTreeMap;

use itertools::Itertools;

pub struct Table {
    columns: Vec<(String, ColumnAlignment)>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub fn new() -> Self {
        Self {
            columns: vec![],
            rows: vec![],
        }
    }

    pub fn with_columns(self, columns: &[(&str, ColumnAlignment)]) -> Self {
        Self {
            columns: columns.iter().map(|(name, alignment)| (name.to_string(), *alignment)).collect(),
            ..self
        }
    }

    pub fn with_rows(self, rows: Vec<Vec<String>>) -> Self {
        Self { rows, ..self }
    }

    pub fn to_table(self) -> tabular::Table {
        let mut longest_per_column = BTreeMap::new();
        let mut table = tabular::Table::new(&format!(
            "| {} |",
            self.columns.iter().map(|(_, alignment)| alignment.to_string()).join(" | ")
        ));
        table.add_row(self.columns.iter().enumerate().fold(tabular::Row::new(), |acc, (i, (data, _))| {
            if let Some(curr_longest) = longest_per_column.get_mut(&i) {
                if *curr_longest < data.len() {
                    *curr_longest = data.len();
                }
            } else {
                longest_per_column.insert(i, data.len());
            };
            acc.with_cell(data)
        }));

        let mut rows = Vec::new();
        for row in self.rows {
            rows.push(row.iter().enumerate().fold(tabular::Row::new(), |acc, (i, data)| {
                if let Some(curr_longest) = longest_per_column.get_mut(&i) {
                    if *curr_longest < data.len() {
                        *curr_longest = data.len();
                    }
                } else {
                    longest_per_column.insert(i, data.len());
                };
                acc.with_cell(data)
            }));
        }

        // Write the dots
        table.add_row(
            longest_per_column
                .values()
                .fold(tabular::Row::new(), |acc, value| acc.with_cell(".".repeat(*value))),
        );

        for row in rows {
            table.add_row(row);
        }

        table
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum ColumnAlignment {
    Left,
    Right,
    Middle,
}

impl ToString for ColumnAlignment {
    fn to_string(&self) -> String {
        match self {
            ColumnAlignment::Left => "{:<}".to_string(),
            ColumnAlignment::Right => "{:>}".to_string(),
            ColumnAlignment::Middle => "{:^}".to_string(),
        }
    }
}
