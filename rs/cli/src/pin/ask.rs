use dialoguer::Password;
use ic_canisters::parallel_hardware_identity::{HsmPinHandler, PinHandlerError};

/// `keyring`-based memory for PIN that uses console prompting for the PIN
/// when a PIN is necessary and has not been supplied by the user.
/// This implementation lives in this crate to avoid having to add a
/// `keyring` dependency to the parallel_hardware_identity module.
#[derive(Default)]
pub struct AskEveryTimePinHandler {}

impl HsmPinHandler for AskEveryTimePinHandler {
    fn retrieve(&self, _key: &str, _from_memory: bool) -> Result<String, PinHandlerError> {
        match Password::new().with_prompt("Please enter the hardware security module PIN: ").interact() {
            Ok(pin) => Ok(pin),
            Err(e) => Err(PinHandlerError(format!("Prompt for PIN failed: {}", e))),
        }
    }

    fn store(&self, _key: &str, _pin: &str) -> Result<(), PinHandlerError> {
        Ok(())
    }

    fn forget(&self, _key: &str) -> Result<(), PinHandlerError> {
        Ok(())
    }
}
