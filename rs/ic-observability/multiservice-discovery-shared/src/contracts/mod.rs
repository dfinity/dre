pub mod deployed_sns;
pub mod journald_target;
pub mod target;

pub trait DataContract {
    fn get_name(&self) -> String;
    fn get_id(&self) -> String;
    fn get_target_name(&self) -> String;
}
