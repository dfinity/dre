use comfy_table::CellAlignment;
use itertools::Itertools;

pub struct Table {
    columns: Vec<(String, CellAlignment)>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub fn new() -> Self {
        Self {
            columns: vec![],
            rows: vec![],
        }
    }

    pub fn with_columns(self, columns: &[(&str, CellAlignment)]) -> Self {
        Self {
            columns: columns.iter().map(|(name, alignment)| (name.to_string(), *alignment)).collect(),
            ..self
        }
    }

    pub fn with_rows(self, rows: Vec<Vec<String>>) -> Self {
        Self { rows, ..self }
    }

    pub fn to_table(&self) -> comfy_table::Table {
        let mut table = comfy_table::Table::new();

        let alignments = self.columns.iter().map(|(_, a)| a).collect_vec();
        let mut header = comfy_table::Row::new();
        for (t, a) in &self.columns {
            header.add_cell(comfy_table::Cell::new(t).set_alignment(*a));
        }
        table.set_header(header);

        for current_row in &self.rows {
            let mut row = comfy_table::Row::new();
            for (t, a) in current_row.iter().zip(alignments.iter()) {
                row.add_cell(comfy_table::Cell::new(t).set_alignment(**a));
            }
            table.add_row(row);
        }

        table
    }
}
