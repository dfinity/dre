use dialoguer::Password;
use ic_canisters::parallel_hardware_identity::{HsmPinHandler, PinHandlerError};
use keyring::{Entry, Error};
use log::error;

/// `keyring`-based memory for PIN that uses console prompting for the PIN
/// when a PIN is necessary and has not been supplied by the user.
/// This implementation lives in this crate to avoid having to add a
/// `keyring` dependency to the parallel_hardware_identity module.
#[derive(Default)]
pub struct PlatformKeyringPinHandler {}

impl HsmPinHandler for PlatformKeyringPinHandler {
    fn retrieve(&self, key: &str, from_memory: bool) -> Result<String, PinHandlerError> {
        if !from_memory {
            match Password::new().with_prompt("Please enter the hardware security module PIN: ").interact() {
                Ok(pin) => Ok(pin),
                Err(e) => Err(PinHandlerError(format!("Prompt for PIN failed: {}", e))),
            }
        } else {
            let entry = match Entry::new("dre-tool-hsm-pin", key) {
                Ok(entry) => Ok(entry),
                Err(e) => Err(PinHandlerError(format!("Keyring initialization failed: {}", e))),
            }?;
            match entry.get_password() {
                Ok(pin) => Ok(pin),
                Err(e) => {
                    if let Error::NoEntry = e {
                    } else {
                        error!("Cannot retrieve password from keyring; switching to unconditional prompt.  Error: {}", e);
                    };
                    match Password::new().with_prompt("Please enter the hardware security module PIN: ").interact() {
                        Ok(pin) => Ok(pin),
                        Err(e) => Err(PinHandlerError(format!("Prompt for PIN failed: {}", e))),
                    }
                }
            }
        }
    }

    fn store(&self, key: &str, pin: &str) -> Result<(), PinHandlerError> {
        let entry = match Entry::new("dre-tool-hsm-pin", key) {
            Ok(entry) => Ok(entry),
            Err(e) => Err(PinHandlerError(format!("Keyring initialization failed: {}", e))),
        }?;
        match entry.set_password(pin) {
            Ok(()) => Ok(()),
            Err(e) => Err(PinHandlerError(format!("{}", e))),
        }
    }

    fn forget(&self, key: &str) -> Result<(), PinHandlerError> {
        let entry = match Entry::new("dre-tool-hsm-pin", key) {
            Ok(entry) => Ok(entry),
            Err(e) => Err(PinHandlerError(format!("Keyring initialization failed: {}", e))),
        }?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(PinHandlerError(format!("{}", e))),
        }
    }
}
