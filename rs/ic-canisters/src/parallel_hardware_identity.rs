use byteorder::{BigEndian, ReadBytesExt};
use cryptoki::{
    context::{CInitializeArgs, Pkcs11 as CryptokiPkcs11},
    error::{Error as CryptokiError, RvError},
    mechanism::Mechanism,
    object::{Attribute, AttributeInfo, AttributeType, KeyType},
    session::UserType,
    slot::{Slot, SlotInfo, TokenInfo},
};
use ic_agent::{agent::EnvelopeContent, export::Principal, identity::Delegation, Identity, Signature};
use log::error;
use log::info;
use log::{debug, warn};
use sha2::{
    digest::{generic_array::GenericArray, OutputSizeUser},
    Digest, Sha256,
};
use simple_asn1::{
    from_der, oid, to_der,
    ASN1Block::{BitString, ObjectIdentifier, OctetString, Sequence},
    ASN1DecodeErr, ASN1EncodeErr,
};
use std::io::Cursor;
use std::{error::Error, sync::Mutex};
use std::{marker::PhantomData, path::Path, str::FromStr, sync::Arc};
use thiserror::Error;

pub type KeyIdVec = Vec<u8>;

#[derive(Clone, Debug, Eq, PartialEq)]
struct HsmAuthParams {
    pub pin: String,
    pub slot: u64,
    pub key_id: KeyIdVec,
}

type DerPublicKeyVec = Vec<u8>;

/// Type alias for a sha256 result (ie. a u256).
type Sha256Hash = GenericArray<u8, <Sha256 as OutputSizeUser>::OutputSize>;

// We expect the curve to be an EC curve.
const EXPECTED_KEY_TYPE: KeyType = KeyType::EC;

// We expect the parameters to be curve secp256r1.  This is the base127 encoded form:
const EXPECTED_EC_PARAMS: &[u8; 10] = b"\x06\x08\x2a\x86\x48\xce\x3d\x03\x01\x07";

// The key ID stored in the HSM is referenced by a sixteen-bit unsigned number.
//  We represent this internally as an array of two bytes.
pub fn hsm_key_id_to_string(s: &KeyIdVec) -> String {
    format!("0x{}", hex::encode(s))
}

// When ic-admin wants the key ID, it is usually to pass to pkcs11-tool's
// --id argument.  That wants the key ID as an integer.
// FIXME: there should be no need to unwrap() here.  The fix is that KeyIdVec
// should simply be a type that contains an u16, and then we don't need to use
// read_uint() here at all.  Will fix in a later PR.
pub fn hsm_key_id_to_int(s: &KeyIdVec) -> String {
    let mut rdr = Cursor::new(s);
    let i = rdr.read_uint::<BigEndian>(s.len()).unwrap();
    format!("{}", i)
}

/// An error happened related to a HardwareIdentity.
#[derive(Error, Debug)]
pub enum HardwareIdentityError {
    /// A PKCS11 error loading the library occurred.
    #[error(transparent)]
    LibraryNotFound(#[from] std::io::Error),

    /// A cryptoki error occurred.
    #[error(transparent)]
    Cryptoki(#[from] cryptoki::error::Error),

    // ASN1DecodeError does not implement the Error trait and so we cannot use #[from]
    /// An error occurred when decoding ASN1.
    #[error("ASN decode error {0}")]
    ASN1Decode(ASN1DecodeErr),

    /// An error occurred when encoding ASN1.
    #[error(transparent)]
    ASN1Encode(#[from] ASN1EncodeErr),

    /// An error occurred when decoding a key ID.
    #[error(transparent)]
    KeyIdDecode(#[from] hex::FromHexError),

    /// The key was not found.
    #[error("Key not found")]
    KeyNotFound,

    /// An EcPoint block was expected to be an OctetString, but was not.
    #[error("Expected EcPoint to be an OctetString")]
    ExpectedEcPointOctetString,

    /// An EcPoint block was expected to be an OctetString, but was not.
    #[error("Expected EcPoint OctetString to be 65 bytes in length, not {0}")]
    IncorrectEcPointLength(usize),

    /// An EcPoint block was unexpectedly empty.
    #[error("EcPoint is empty")]
    EcPointEmpty,

    /// The attribute with the specified type was not found.
    #[error("Attribute with type={0} not found")]
    AttributeNotFound(AttributeType),

    /// The EcParams given were not the ones the crate expected.
    #[error("Invalid EcParams.  Expected prime256v1 {:02x?}, actual is {:02x?}", .expected, .actual)]
    InvalidEcParams {
        /// The expected value of the EcParams.
        expected: Vec<u8>,
        /// The actual value of the EcParams.
        actual: Vec<u8>,
    },

    /// The PIN login function returned an error, but PIN login was required.
    #[error("User PIN is required: {0}")]
    UserPinRequired(String),

    /// The PIN is expired.
    #[error("User PIN expired")]
    UserPinExpired,

    /// The PIN is not correct.
    #[error("User PIN incorrect")]
    UserPinIncorrect,

    /// The PIN is not valid.
    #[error("User PIN invalid")]
    UserPinInvalid,

    /// The PIN is too long or too short.
    #[error("User PIN too long or too short")]
    UserPinLenRange,

    /// The PIN is too long or too short.
    #[error("The user PIN for the token in slot {0} is locked")]
    UserPinLocked(Slot),

    /// The PIN is too long or too short.
    #[error("User PIN does not meet the strength standards")]
    UserPinTooWeak,

    /// A slot index was provided that does not exist.
    #[error("No such slot index ({0}")]
    NoSuchSlotIndex(usize),

    #[error("Hardware security key not found: {0}")]
    NoHSM(String),
}

fn pkcs11_lib_path() -> Result<std::path::PathBuf, std::io::Error> {
    let lib_macos_path = std::path::PathBuf::from_str("/Library/OpenSC/lib/opensc-pkcs11.so").unwrap();
    let lib_linux_path = std::path::PathBuf::from_str("/usr/lib/x86_64-linux-gnu/opensc-pkcs11.so").unwrap();
    if lib_macos_path.exists() {
        Ok(lib_macos_path)
    } else if lib_linux_path.exists() {
        Ok(lib_linux_path)
    } else {
        Err(std::io::Error::from(std::io::ErrorKind::NotFound))
    }
}

#[derive(Debug)]
struct PKCS11Ctx {
    cryptokictx: CryptokiPkcs11,
}

impl PKCS11Ctx {
    fn init() -> Result<Self, HardwareIdentityError> {
        let p = pkcs11_lib_path().map_err(HardwareIdentityError::LibraryNotFound)?;
        debug!(target: "pkcs11", "PKCS#11 context is being initialized");
        Self::init_with_path(p)
    }

    fn init_with_path<P>(pkcs11_lib_path: P) -> Result<Self, HardwareIdentityError>
    where
        P: AsRef<Path> + Clone,
    {
        let cryptokictx = CryptokiPkcs11::new(pkcs11_lib_path.clone())?;
        cryptokictx.initialize(CInitializeArgs::OsThreads)?;
        Ok(Self { cryptokictx })
    }

    fn get_slots_with_token(&self) -> Result<Vec<Slot>, HardwareIdentityError> {
        Ok(self.cryptokictx.get_slots_with_token()?)
    }

    fn get_slot_info(&self, slot: Slot) -> Result<SlotInfo, HardwareIdentityError> {
        Ok(self.cryptokictx.get_slot_info(slot)?)
    }

    fn get_token_info(&self, slot: Slot) -> Result<TokenInfo, HardwareIdentityError> {
        Ok(self.cryptokictx.get_token_info(slot)?)
    }

    fn open_ro_session(&self, slot: Slot) -> Result<PKCS11Sess<Unauthenticated>, CryptokiError> {
        let sess = self.cryptokictx.open_ro_session(slot)?;
        let s: PKCS11Sess<Unauthenticated> = PKCS11Sess {
            sess,
            slot,
            _marker: PhantomData,
        };
        Ok(s)
    }
}

impl Drop for PKCS11Ctx {
    fn drop(&mut self) {
        debug!(target: "pkcs11", "PKCS#11 context is being retired");
    }
}

trait IdentityState {}
struct Unauthenticated;
struct Authenticated;

impl IdentityState for Unauthenticated {}
impl IdentityState for Authenticated {}

struct PKCS11Sess<S: IdentityState> {
    sess: cryptoki::session::Session,
    slot: Slot,
    _marker: PhantomData<S>,
}

impl<S: IdentityState> PKCS11Sess<S> {
    /// Finds the key ID in a slot.  If a key ID is specified,
    /// then the search is limited to that key ID.  If not, then
    /// the first key that has an ID and is for a token is returned.
    /// If a key is found, this function returns Some, with a tuple of
    /// the found key ID, and possibly the label assigned to said key ID
    /// (None if no / invalid label).
    fn find_key_id(&self, key_id: Option<KeyIdVec>) -> Result<Option<(KeyIdVec, Option<String>)>, HardwareIdentityError> {
        let token_types = vec![AttributeType::Token, AttributeType::Id];
        let label_types = vec![AttributeType::Label];
        let objects = self.sess.find_objects(&[])?;
        for hnd in objects.iter() {
            if let [AttributeInfo::Available(_), AttributeInfo::Available(_)] =
                self.sess.get_attribute_info(*hnd, &token_types)?[0..token_types.len()]
            {
                // Object may be a token and has an ID.
                if let [Attribute::Token(true), Attribute::Id(token_id)] = &self.sess.get_attributes(*hnd, &token_types)?[0..token_types.len()] {
                    // Object is a token, and we have extracted the ID.
                    if !token_id.is_empty()
                        && match &key_id {
                            None => true,
                            Some(key_id) => *token_id == *key_id,
                        }
                    {
                        let found_key_id = token_id;
                        let mut label: Option<String> = None;
                        if let [AttributeInfo::Available(_)] = &self.sess.get_attribute_info(*hnd, &label_types)?[0..label_types.len()] {
                            // Object has a label.
                            if let [Attribute::Label(token_label)] = &self.sess.get_attributes(*hnd, &label_types)?[0..label_types.len()] {
                                // We have extracted the label; we make a copy of it.
                                label = match String::from_utf8(token_label.clone()) {
                                    Ok(label) => Some(label),
                                    Err(_) => None,
                                }
                            }
                        }
                        return Ok(Some((found_key_id.clone(), label)));
                    }
                }
            }
        }
        Ok(None)
    }

    fn get_der_encoded_public_key(&self, key_id: KeyIdVec) -> Result<DerPublicKeyVec, HardwareIdentityError> {
        // Obtain all ECDSA keys that match the expected EC parameters.
        let key_handle = *self
            .sess
            .find_objects(&[
                Attribute::Id(key_id),
                Attribute::KeyType(EXPECTED_KEY_TYPE),
                Attribute::EcParams(EXPECTED_EC_PARAMS.to_vec()),
            ])?
            .first()
            .ok_or(HardwareIdentityError::KeyNotFound)?;

        let der_encoded_ec_point = match self
            .sess
            .get_attributes(key_handle, &[AttributeType::EcPoint])?
            .first()
            .ok_or(HardwareIdentityError::EcPointEmpty)?
        {
            Attribute::EcPoint(point) => point.clone(),
            _ => return Err(HardwareIdentityError::AttributeNotFound(AttributeType::EcPoint)),
        };

        let asn1_blocks = from_der(der_encoded_ec_point.as_slice()).map_err(HardwareIdentityError::ASN1Decode)?;
        let asn1_block = asn1_blocks.first().ok_or(HardwareIdentityError::EcPointEmpty)?;

        let ec_point = if let OctetString(_size, data) = asn1_block {
            Ok(data.clone())
        } else {
            Err(HardwareIdentityError::ExpectedEcPointOctetString)
        }?;

        // We expect the octet string to be 65 bytes in length.
        if ec_point.len() != 65 {
            return Err(HardwareIdentityError::IncorrectEcPointLength(ec_point.len()));
        }
        let (oid_ecdsa, oid_curve_secp256r1) = (oid!(1, 2, 840, 10045, 2, 1), oid!(1, 2, 840, 10045, 3, 1, 7));
        let asn1_public_key = Sequence(
            0,
            vec![
                Sequence(0, vec![ObjectIdentifier(0, oid_ecdsa), ObjectIdentifier(0, oid_curve_secp256r1)]),
                BitString(0, ec_point.len() * 8, ec_point),
            ],
        );
        Ok(to_der(&asn1_public_key)?)
    }
}

impl PKCS11Sess<Unauthenticated> {
    fn authenticate(self, pin: String) -> Result<PKCS11Sess<Authenticated>, HardwareIdentityError> {
        let pin = cryptoki::types::AuthPin::from_str(pin.as_str()).unwrap();
        match self.sess.login(UserType::User, Some(&pin)) {
            Ok(_) => (),
            Err(CryptokiError::Pkcs11(RvError::PinExpired, _)) => return Err(HardwareIdentityError::UserPinExpired),
            Err(CryptokiError::Pkcs11(RvError::PinIncorrect, _)) => return Err(HardwareIdentityError::UserPinIncorrect),
            Err(CryptokiError::Pkcs11(RvError::PinInvalid, _)) => return Err(HardwareIdentityError::UserPinInvalid),
            Err(CryptokiError::Pkcs11(RvError::PinLenRange, _)) => return Err(HardwareIdentityError::UserPinLenRange),
            Err(CryptokiError::Pkcs11(RvError::PinLocked, _)) => return Err(HardwareIdentityError::UserPinLocked(self.slot)),
            Err(CryptokiError::Pkcs11(RvError::PinTooWeak, _)) => return Err(HardwareIdentityError::UserPinTooWeak),
            Err(e) => return Err(HardwareIdentityError::Cryptoki(e)),
        };
        let sess: PKCS11Sess<Authenticated> = PKCS11Sess {
            sess: self.sess,
            slot: self.slot,
            _marker: PhantomData,
        };
        Ok(sess)
    }
}

impl PKCS11Sess<Authenticated> {
    fn sign_hash(&self, key_id: KeyIdVec, hash: &Sha256Hash) -> Result<Vec<u8>, HardwareIdentityError> {
        let key_handle = *self
            .sess
            .find_objects(&[
                Attribute::Id(key_id),
                Attribute::KeyType(EXPECTED_KEY_TYPE),
                Attribute::EcParams(EXPECTED_EC_PARAMS.to_vec()),
            ])?
            .first()
            .ok_or(HardwareIdentityError::KeyNotFound)?;
        let mechanism = Mechanism::Ecdsa;
        self.sess.sign(&mechanism, key_handle, hash).map_err(|e| e.into())
    }
}

pub struct DetectedHsm {
    ctx: PKCS11Ctx,
    sess: PKCS11Sess<Unauthenticated>,
    token_info: TokenInfo,
    key_id: KeyIdVec,
    pub memo_key: String,
}

impl DetectedHsm {
    pub fn authenticate(self, pin: String) -> Result<ParallelHardwareIdentity, HardwareIdentityError> {
        if self.token_info.user_pin_locked() {
            return Err(HardwareIdentityError::UserPinLocked(self.sess.slot));
        }
        if self.token_info.user_pin_final_try() {
            warn!(
                "The PIN for the token stored in slot {} is at its last try, and if this operation fails, the token will be locked",
                self.sess.slot
            );
        }
        let slot_u64: u64 = self.sess.slot.into();
        self.sess.authenticate(pin.clone())?;
        info!("Hardware security module PIN correct");
        ParallelHardwareIdentity::new(
            self.ctx,
            HsmAuthParams {
                key_id: self.key_id.clone(),
                slot: slot_u64,
                pin,
            },
        )
    }
}

/// An identity based on an HSM.
#[derive(Debug, Clone)]
pub struct ParallelHardwareIdentity {
    pub key_id: KeyIdVec,
    ctx: Arc<Mutex<Option<PKCS11Ctx>>>,
    public_key: DerPublicKeyVec,
    pub slot: Slot,
    // FIXME: This should be a criptoki::secret to prevent debug dumps of it.
    pub cached_pin: String,
}

impl ParallelHardwareIdentity {
    /// Create an identity using a specific key on an HSM, through a PKCS#11 context.
    /// The key_id must refer to a ECDSA key with parameters prime256v1 (secp256r1)
    /// The key must already have been created.  You can create one with pkcs11-tool:
    /// $ pkcs11-tool -k --slot $SLOT -d $KEY_ID --key-type EC:prime256v1 --pin $PIN
    fn new(ctx: PKCS11Ctx, params: HsmAuthParams) -> Result<Self, HardwareIdentityError> {
        let slot_id = params.slot;
        let key_id = params.key_id.clone();

        let slot: Slot = slot_id.try_into()?;
        let public_key = match ctx.open_ro_session(slot) {
            Ok(sess) => sess,
            Err(cryptoki::error::Error::Pkcs11(RvError::SlotIdInvalid, _)) => return Err(HardwareIdentityError::NoSuchSlotIndex(slot_id as usize)),
            Err(e) => return Err(HardwareIdentityError::Cryptoki(e)),
        }
        .get_der_encoded_public_key(key_id.clone())?;

        Ok(Self {
            key_id,
            ctx: Arc::new(Mutex::new(Some(ctx))),
            public_key,
            slot,
            cached_pin: params.pin,
        })
    }

    pub fn scan_for_hsm(maybe_slot: Option<u64>, maybe_key_id: Option<KeyIdVec>) -> Result<DetectedHsm, HardwareIdentityError> {
        let ctx = PKCS11Ctx::init()?;

        if maybe_slot.is_none() && maybe_key_id.is_none() {
            debug!("Scanning hardware security module devices");
        }
        if let Some(slot) = &maybe_slot {
            debug!("Probing hardware security module in slot {}", slot);
        }
        if let Some(key_id) = &maybe_key_id {
            debug!("Limiting key scan to keys with ID {}", hsm_key_id_to_string(key_id));
        }

        for slot in ctx.get_slots_with_token()? {
            let info = ctx.get_slot_info(slot)?;
            let token_info = ctx.get_token_info(slot)?;
            if info.slot_description().starts_with("Nitrokey Nitrokey HSM") && maybe_slot.is_none() || (maybe_slot.unwrap() == slot.id()) {
                let sess = ctx.open_ro_session(slot)?;
                let key_id = match sess.find_key_id(maybe_key_id.clone())? {
                    Some((key_id, label)) => {
                        debug!(
                            "Found key with ID {} ({}) in slot {}",
                            hsm_key_id_to_string(&key_id),
                            match label {
                                Some(label) => format!("labeled {}", label),
                                None => "without label".to_string(),
                            },
                            slot.id()
                        );
                        key_id
                    }
                    None => {
                        if maybe_slot.is_some() && maybe_key_id.is_some() {
                            // We have been asked to be very specific.  Fail fast,
                            // instead of falling back to Auth::Anonymous.
                            return Err(HardwareIdentityError::NoHSM(format!(
                                "Could not find a key ID {} within hardware security module in slot {}",
                                hsm_key_id_to_string(&maybe_key_id.unwrap()),
                                slot.id()
                            )));
                        } else {
                            // Let's try the next slot just in case.
                            continue;
                        }
                    }
                };
                let memo_key: String = format!("hsm-{}-{}", info.slot_description(), info.manufacturer_id());
                info!(
                    "Trying key ID {} of hardware security module in slot {} ({})",
                    hsm_key_id_to_string(&key_id),
                    slot,
                    memo_key
                );
                return Ok(DetectedHsm {
                    ctx,
                    sess,
                    token_info,
                    key_id,
                    memo_key,
                });
            }
        }
        Err(HardwareIdentityError::NoHSM(format!(
            "No hardware security module detected{}{}",
            match maybe_slot {
                None => "".to_string(),
                Some(slot) => format!(" in slot {}", slot),
            },
            match &maybe_key_id {
                None => "".to_string(),
                Some(key_id) => format!(" that contains a key ID {}", hsm_key_id_to_string(key_id)),
            }
        )))
    }
}

impl Identity for ParallelHardwareIdentity {
    fn sender(&self) -> Result<Principal, String> {
        Ok(Principal::self_authenticating(&self.public_key))
    }

    fn public_key(&self) -> Option<Vec<u8>> {
        Some(self.public_key.clone())
    }

    fn sign(&self, content: &EnvelopeContent) -> Result<Signature, String> {
        self.sign_arbitrary(&content.to_request_id().signable())
    }

    fn sign_delegation(&self, content: &Delegation) -> Result<Signature, String> {
        self.sign_arbitrary(&content.signable())
    }

    fn sign_arbitrary(&self, content: &[u8]) -> Result<Signature, String> {
        let hash = Sha256::digest(content);
        let signature = self.sign_hash(&hash).map_err(|e| {
            format!(
                "Could not sign payload: {}{}",
                e,
                match e.source() {
                    None => "".to_string(),
                    Some(source) => format!(" ({})", source),
                }
            )
        })?;
        Ok(Signature {
            public_key: self.public_key(),
            signature: Some(signature),
            delegations: None,
        })
    }
}

impl ParallelHardwareIdentity {
    fn sign_hash_inner(&self, ctx: &PKCS11Ctx, hash: &Sha256Hash) -> Result<Vec<u8>, HardwareIdentityError> {
        let sess = ctx.open_ro_session(self.slot)?;
        let signer = sess.authenticate(self.cached_pin.clone())?;
        signer.sign_hash(self.key_id.clone(), hash)
    }

    /// Sign a particular SHA256 hash.  If there is a temporary error during signing,
    /// this will attempt to recreate the PKCS#11 context and try signing again.
    fn sign_hash(&self, hash: &Sha256Hash) -> Result<Vec<u8>, HardwareIdentityError> {
        let mut ctx_guard = self.ctx.lock().unwrap();
        // The PKCS#11 context may have been deleted before.
        // If empty, it must be recreated.
        let ctx = match ctx_guard.take() {
            None => {
                warn!(target:"pkcs11", "PKCS#11 context appears to have been dropped before; will initialize a new context now");
                PKCS11Ctx::init()?
            }
            Some(c) => c,
        };
        match self.sign_hash_inner(&ctx, hash) {
            // Signature succeeded.
            Ok(signature) => {
                ctx_guard.replace(ctx);
                // Stow away the context before returning the result.
                Ok(signature)
            }
            // Uh oh, failure.  Try deleting the context and recreating it, before
            // retrying the signing operation.
            Err(e) => {
                warn!(target:"pkcs11", "PKCS#11 context appears to be malfunctioning ({}); will reset the context now and retry the signing operation", e);
                drop(ctx);
                let new_ctx = match PKCS11Ctx::init() {
                    Ok(c) => Ok(c),
                    Err(_) => Err(e),
                }?;
                let ret = self.sign_hash_inner(&new_ctx, hash);
                // Stow away the context we just used.
                ctx_guard.replace(new_ctx);
                ret
            }
        }
    }
}
