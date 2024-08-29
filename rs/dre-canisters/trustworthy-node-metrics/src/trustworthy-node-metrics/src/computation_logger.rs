use std::fmt;

use itertools::Itertools;
use num_traits::Zero;
use rust_decimal::Decimal;

pub enum Operation {
    Set(Decimal),
    Sum(Vec<Decimal>),
    Subtract(Decimal, Decimal),
    Divide(Decimal, Decimal),
}

impl Operation {
    fn execute(&self) -> Decimal {
        match self {
            Operation::Sum(operators) => operators.iter().cloned().fold(Decimal::zero(), |acc, val| acc + val),
            Operation::Subtract(o1, o2) => o1 - o2,
            Operation::Divide(o1, o2) => o1/o2,
            Operation::Set(o1) => *o1,
        }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (symbol, o1, o2) = match self {
            Operation::Sum(values) => {
                return write!(
                    f,
                    "{} + {}",
                    values[0],
                    values[1..].iter().map(ToString::to_string).collect::<Vec<_>>().join(" + ")
                )
            }
            Operation::Subtract(o1, o2) => ("-", o1, o2),
            Operation::Divide(o1, o2) => return write!(f, "{} / {}", o1, o2),
            Operation::Set(o1) => return write!(f, "{}", o1),
        };
        write!(f, "{} {} {}", o1, symbol, o2)
    }
}

pub struct OperationExecutor {
    reason: String,
    operation: Operation,
    result: Decimal,
}

impl OperationExecutor {
    pub fn execute(reason: &str, operation: Operation) -> (Self, Decimal) {
        let result = operation.execute();

        let operation_executed = Self {
            reason: reason.to_string(),
            operation,
            result,
        };

        (operation_executed, result)
    }
}

impl fmt::Display for OperationExecutor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}: {} = {}", self.reason, self.operation, self.result)?;
        Ok(())
    }
}

// Modify ComputationLogger to use NumberEnum
pub struct ComputationLogger {
    maybe_input: Option<String>,
    operations_executed: Vec<OperationExecutor>,
}

impl ComputationLogger {
    pub fn new() -> Self {
        Self {
            maybe_input: None,
            operations_executed: Vec::new(),
        }
    }

    pub fn with_input(self, input: String) -> Self {
        Self {
            maybe_input: Some(input),
            ..self
        }
    }

    pub fn execute(&mut self, reason: &str, operation: Operation) -> Decimal {
        let result = operation.execute();

        let operation_executed = OperationExecutor {
            reason: reason.to_string(),
            operation,
            result,
        };
        self.operations_executed.push(operation_executed);
        result
    }

    pub fn add_executed(&mut self, operations: Vec<OperationExecutor>) {
        for operation in operations {
            self.operations_executed.push(operation)
        }
    }

    pub fn get_log(&self) -> String {
        let operations_log = self
            .operations_executed
            .iter()
            .enumerate()
            .map(|(index, item)| format!("STEP {}: {}", index + 1, item))
            .collect_vec()
            .join("\n");

        if let Some(input) = &self.maybe_input {
            format!("INPUT:\n{}\nCOMPUTATION LOG:\n\n{}", input, operations_log)
        } else {
            format!("COMPUTATION LOG:\n\n{}", operations_log)
        }
    }
}
