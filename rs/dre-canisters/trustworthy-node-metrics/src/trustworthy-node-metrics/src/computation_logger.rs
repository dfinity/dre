use itertools::Itertools;
use num_traits::{FromPrimitive, Num, ToPrimitive};
use std::fmt::{self, Display};

// Define a trait that will be used to store and manipulate numbers
pub trait Number: Num + Copy + std::fmt::Debug + Display + ToPrimitive + FromPrimitive + std::iter::Sum + std::cmp::PartialOrd {}

impl Number for u64 {}

pub enum Operation<T> {
    Set(T),
    Sum(Vec<T>),
    Subtract(T, T),
    Percent(T, T),
}

impl<T: Number> Operation<T> {
    fn execute(&self) -> T {
        match self {
            Operation::Sum(operators) => operators.iter().cloned().sum(),
            Operation::Subtract(o1, o2) => *o1 - *o2,
            Operation::Percent(o1, o2) => {
                assert!(o1 <= o2, "Percent operation requires o1 <= o2");
                assert!(!T::is_zero(o2), "Division by 0");
                let numerator = o1.to_f64().unwrap();
                let denominator = o2.to_f64().unwrap();

                let result = (numerator / denominator * 100.0).round();
                FromPrimitive::from_f64(result).unwrap()
            }
            Operation::Set(o1) => *o1,
        }
    }
}

impl<T: Number> fmt::Display for Operation<T> {
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
            Operation::Percent(o1, o2) => return write!(f, "{} / {} * 100", o1, o2),
            Operation::Set(o1) => return write!(f, "{}", o1),
        };
        write!(f, "{} {} {}", o1, symbol, o2)
    }
}

pub struct OperationExecuted<T: Number> {
    reason: String,
    operation: Operation<T>,
    result: T,
}

impl<T: Number> OperationExecuted<T> {
    pub fn execute(reason: &str, operation: Operation<T>) -> (Self, T) {
        let result = operation.execute();

        let operation_executed = Self {
            reason: reason.to_string(),
            operation,
            result,
        };

        (operation_executed, result)
    }
}

impl<T: Number> fmt::Display for OperationExecuted<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}: {} = {}", self.reason, self.operation, self.result)?;
        Ok(())
    }
}

pub struct ComputationLogger<T: Number> {
    maybe_input: Option<String>,
    operations_executed: Vec<OperationExecuted<T>>,
}

impl<T: Number> ComputationLogger<T> {
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

    pub fn execute(&mut self, reason: &str, operation: Operation<T>) -> T {
        let result = operation.execute();

        let operation_executed = OperationExecuted {
            reason: reason.to_string(),
            operation,
            result,
        };
        self.operations_executed.push(operation_executed);
        result
    }

    pub fn add_executed(&mut self, operations: Vec<OperationExecuted<T>>) {
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
