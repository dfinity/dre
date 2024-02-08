use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use futures_util::future::join_all;
use ic_management_types::Network;
use ic_registry_client::client::ThresholdSigPublicKey;
use opentelemetry::KeyValue;
use service_discovery::job_types::JobType;
use service_discovery::registry_sync::Interrupted;
use service_discovery::IcServiceDiscovery;
use service_discovery::IcServiceDiscoveryError;
use service_discovery::TargetGroup;
use service_discovery::{registry_sync::sync_local_registry, IcServiceDiscoveryImpl};
use slog::{debug, info, warn, Logger};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::Debug;
use std::fmt::{Display, Error as FmtError, Formatter};
use std::net::SocketAddr;
use std::sync::Arc;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use url::Url;

use crate::metrics::RunningDefinitionsMetrics;

#[derive(Clone)]
pub struct Definition {
    pub nns_urls: Vec<Url>,
    pub registry_path: PathBuf,
    pub name: String,
    log: Logger,
    pub public_key: Option<ThresholdSigPublicKey>,
    pub poll_interval: Duration,
    pub registry_query_timeout: Duration,
    pub ic_discovery: Arc<IcServiceDiscoveryImpl>,
    pub boundary_nodes: Vec<BoundaryNode>,
}

impl Debug for Definition {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(
            f,
            "Definition <
    name: {},
    nns_urls: {:?}
    public_key: {:?}
>",
            self.name, self.nns_urls, self.public_key
        )
    }
}

struct Ender {
    stop_signal_sender: Sender<()>,
    join_handle: std::thread::JoinHandle<()>,
}

#[derive(Debug)]
pub(crate) struct BoundaryNodeAlreadyExists {
    name: String,
}

impl Error for BoundaryNodeAlreadyExists {}

impl Display for BoundaryNodeAlreadyExists {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        write!(f, "boundary node {} already exists", self.name)
    }
}

#[derive(Clone)]
pub struct RunningDefinition {
    pub(crate) definition: Definition,
    stop_signal: Receiver<()>,
    ender: Arc<Mutex<Option<Ender>>>,
    metrics: RunningDefinitionsMetrics,
}

pub struct TestDefinition {
    pub(crate) running_def: RunningDefinition,
}

impl TestDefinition {
    pub(crate) fn new(definition: Definition, metrics: RunningDefinitionsMetrics) -> Self {
        let (_, stop_signal) = crossbeam::channel::bounded::<()>(0);
        let ender: Arc<Mutex<Option<Ender>>> = Arc::new(Mutex::new(None));
        Self {
            running_def: RunningDefinition {
                definition,
                stop_signal,
                ender,
                metrics,
            },
        }
    }

    /// Syncs the registry update the in-memory cache then stops.
    pub async fn sync_and_stop(&self) {
        let _ = self.running_def.initial_registry_sync().await;
        // if self.initial_registry_sync().await.is_err() {
        // FIXME: Error has been logged, but ideally, it should be handled.
        // E.g. telemetry should collect this.
        // return;
        // }
        let _ = self
            .running_def
            .definition
            .ic_discovery
            .load_new_ics(self.running_def.definition.log.clone());
    }
}

impl Definition {
    pub(crate) fn new(
        nns_urls: Vec<Url>,
        global_registry_path: PathBuf,
        name: String,
        log: Logger,
        public_key: Option<ThresholdSigPublicKey>,
        poll_interval: Duration,
        registry_query_timeout: Duration,
    ) -> Self {
        let global_registry_path = std::fs::canonicalize(global_registry_path).expect("Invalid global registry path");
        // The path needs to be sanitized otherwise any file in the environment can be overwritten,
        let sanitized_name = name.replace(['.', '/'], "_");
        let registry_path = global_registry_path.join(sanitized_name);
        if std::fs::metadata(&registry_path).is_err() {
            std::fs::create_dir_all(registry_path.clone()).unwrap();
        }
        Self {
            nns_urls,
            registry_path: registry_path.clone(),
            name,
            log: log.clone(),
            public_key,
            poll_interval,
            registry_query_timeout,
            ic_discovery: Arc::new(IcServiceDiscoveryImpl::new(log, registry_path, registry_query_timeout).unwrap()),
            boundary_nodes: vec![],
        }
    }

    pub(crate) async fn run(self, rt: tokio::runtime::Handle, metrics: RunningDefinitionsMetrics) -> RunningDefinition {
        fn wrap(definition: RunningDefinition, rt: tokio::runtime::Handle) -> impl FnMut() {
            move || {
                rt.block_on(definition.run());
            }
        }

        info!(self.log, "Running new definition {}", self.name);
        let (stop_signal_sender, stop_signal) = crossbeam::channel::bounded::<()>(0);
        let ender: Arc<Mutex<Option<Ender>>> = Arc::new(Mutex::new(None));
        let d = RunningDefinition {
            definition: self,
            stop_signal,
            ender: ender.clone(),
            metrics,
        };
        let join_handle = std::thread::spawn(wrap(d.clone(), rt));
        ender.lock().await.replace(Ender {
            stop_signal_sender,
            join_handle,
        });
        d
    }
}

impl RunningDefinition {
    pub(crate) async fn end(&mut self) {
        let mut ender = self.ender.lock().await;
        if let Some(s) = ender.take() {
            // We have pulled out the channel from its container.  After this,
            // all senders will have been dropped, and no more messages can be sent.
            // https://docs.rs/crossbeam/latest/crossbeam/channel/index.html#disconnection
            info!(
                self.definition.log,
                "Sending termination signal to definition {}", self.definition.name
            );
            s.stop_signal_sender.send(()).unwrap();
            info!(
                self.definition.log,
                "Joining definition {} thread", self.definition.name
            );
            s.join_handle.join().unwrap();
        }
    }

    pub(crate) fn get_target_groups(
        &self,
        job_type: JobType,
    ) -> Result<BTreeSet<TargetGroup>, IcServiceDiscoveryError> {
        self.definition
            .ic_discovery
            .get_target_groups(job_type, self.definition.log.clone())
    }

    async fn initial_registry_sync(&self) -> Result<(), Interrupted> {
        info!(
            self.definition.log,
            "Syncing local registry for {} started", self.definition.name
        );
        info!(
            self.definition.log,
            "Using local registry path: {}",
            self.definition.registry_path.display()
        );

        let r = sync_local_registry(
            self.definition.log.clone(),
            self.definition.registry_path.join("targets"),
            self.definition.nns_urls.clone(),
            false,
            self.definition.public_key,
            &self.stop_signal,
        )
        .await;
        match r {
            Ok(_) => info!(
                self.definition.log,
                "Syncing local registry for {} completed", self.definition.name,
            ),
            Err(_) => warn!(
                self.definition.log,
                "Interrupted initial sync of definition {}", self.definition.name
            ),
        }
        r
    }

    async fn poll_loop(&self) {
        let interval = crossbeam::channel::tick(self.definition.poll_interval);
        let mut tick = Instant::now();
        loop {
            debug!(
                self.definition.log,
                "Loading new scraping targets for {}, (tick: {:?})", self.definition.name, tick
            );
            if let Err(e) = self.definition.ic_discovery.load_new_ics(self.definition.log.clone()) {
                warn!(
                    self.definition.log,
                    "Failed to load new scraping targets for {} @ interval {:?}: {:?}", self.definition.name, tick, e
                );
                self.metrics
                    .load_new_targets_error
                    .add(1, &[KeyValue::new("definition", self.name())])
            }
            debug!(self.definition.log, "Update registries for {}", self.definition.name);
            if let Err(e) = self.definition.ic_discovery.update_registries().await {
                warn!(
                    self.definition.log,
                    "Failed to sync registry for {} @ interval {:?}: {:?}", self.definition.name, tick, e
                );
                self.metrics
                    .sync_registry_error
                    .add(1, &[KeyValue::new("definition", self.name())])
            }

            tick = crossbeam::select! {
                recv(self.stop_signal) -> _ => {
                    info!(self.definition.log, "Received shutdown signal in poll_loop for {}", self.definition.name);
                    return
                },
                recv(interval) -> msg => msg.expect("tick failed!")
            }
        }
    }

    // Syncs the registry and keeps running, syncing as new
    // registry versions come in.
    async fn run(&self) {
        if self.initial_registry_sync().await.is_err() {
            // FIXME: Error has been logged, but ideally, it should be handled.
            // E.g. telemetry should collect this.
            return;
        }

        info!(
            self.definition.log,
            "Starting to watch for changes for definition {}", self.definition.name
        );

        self.poll_loop().await;

        // We used to delete storage here, but that was unsafe
        // because another definition may be started in its name,
        // so it is racy to delete the folder it will be using.
        // So we no longer delete storage here.
    }

    pub(crate) async fn add_boundary_node(&mut self, target: BoundaryNode) -> Result<(), BoundaryNodeAlreadyExists> {
        // Lock modifications to this object while mods are happening.
        match self.ender.lock().await.as_ref() {
            Some(_) => {
                if let Some(bn) = self.definition.boundary_nodes.iter().find(|bn| bn.name == target.name) {
                    return Err(BoundaryNodeAlreadyExists { name: bn.name.clone() });
                };

                self.definition.boundary_nodes.push(target);
                Ok(())
            }
            _ => Ok(()), // Ended.  Do nothing.
        }
    }

    pub fn name(&self) -> String {
        self.definition.name.clone()
    }
}

#[derive(Clone)]
pub struct BoundaryNode {
    pub name: String,
    pub targets: BTreeSet<SocketAddr>,
    pub custom_labels: BTreeMap<String, String>,
    pub job_type: JobType,
}

#[derive(Debug)]
pub(crate) enum StartDefinitionError {
    AlreadyExists(String),
    DeletionDisallowed(String),
}

impl Error for StartDefinitionError {}

impl Display for StartDefinitionError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        match self {
            Self::AlreadyExists(name) => write!(f, "definition {} is already running", name),
            Self::DeletionDisallowed(name) => write!(f, "deletion of {} is disallowed without a replacement", name),
        }
    }
}
#[derive(Debug)]
pub(crate) struct StartDefinitionsError {
    pub(crate) errors: Vec<StartDefinitionError>,
}

impl Error for StartDefinitionsError {}

impl Display for StartDefinitionsError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        for e in self.errors.iter() {
            write!(f, "* {}", e)?
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) enum StopDefinitionError {
    DoesNotExist(String),
    DeletionDisallowed(String),
}

impl Error for StopDefinitionError {}

impl Display for StopDefinitionError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        match self {
            Self::DoesNotExist(name) => write!(f, "definition {} does not exist", name),
            Self::DeletionDisallowed(name) => write!(f, "deletion of {} is disallowed by configuration", name),
        }
    }
}
#[derive(Debug)]
pub(crate) struct StopDefinitionsError {
    pub(crate) errors: Vec<StopDefinitionError>,
}

impl Error for StopDefinitionsError {}

impl Display for StopDefinitionsError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        for e in self.errors.iter() {
            write!(f, "* {}", e)?
        }
        Ok(())
    }
}

#[derive(PartialEq)]
pub(crate) enum StartMode {
    AddToDefinitions,
    ReplaceExistingDefinitions,
}

#[derive(Clone)]
pub(super) struct DefinitionsSupervisor {
    rt: tokio::runtime::Handle,
    pub(super) definitions: Arc<Mutex<BTreeMap<String, RunningDefinition>>>,
    allow_mercury_deletion: bool,
}

impl DefinitionsSupervisor {
    pub(crate) fn new(rt: tokio::runtime::Handle, allow_mercury_deletion: bool) -> Self {
        DefinitionsSupervisor {
            rt,
            definitions: Arc::new(Mutex::new(BTreeMap::new())),
            allow_mercury_deletion,
        }
    }

    async fn start_inner(
        &self,
        existing: &mut BTreeMap<String, RunningDefinition>,
        definitions: Vec<Definition>,
        start_mode: StartMode,
        metrics: RunningDefinitionsMetrics,
    ) -> Result<(), StartDefinitionsError> {
        let mut error = StartDefinitionsError { errors: vec![] };
        let mut ic_names_to_add: HashSet<String> = HashSet::new();

        for definition in definitions.iter() {
            let ic_name = definition.name.clone();
            // Check if we already have something running with the same name,
            // if the user does not want to replace those with newer defs.
            if start_mode == StartMode::AddToDefinitions && existing.contains_key(&ic_name) {
                error.errors.push(StartDefinitionError::AlreadyExists(ic_name.clone()));
                continue;
            }

            // Check for incoming duplicates.
            if ic_names_to_add.contains(&ic_name) {
                error.errors.push(StartDefinitionError::AlreadyExists(ic_name.clone()));
                continue;
            }
            ic_names_to_add.insert(ic_name);
        }

        if !self.allow_mercury_deletion
            && !ic_names_to_add.contains(&Network::Mainnet.legacy_name())
            && start_mode == StartMode::ReplaceExistingDefinitions
        {
            error
                .errors
                .push(StartDefinitionError::DeletionDisallowed(Network::Mainnet.legacy_name()))
        }

        if !error.errors.is_empty() {
            return Err(error);
        }

        // We stop X before we start X' because otherwise
        // the newly-running definition will fight over
        // shared disk space (a folder) and probably die.
        let ic_names_to_end: Vec<String> = existing
            .clone()
            .into_keys()
            .filter(|ic_name| match start_mode {
                // In this mode, we only remove existing definitions if they are going to be replaced.
                StartMode::AddToDefinitions => ic_names_to_add.contains(ic_name),
                // In this mode, we remove all definitions.
                StartMode::ReplaceExistingDefinitions => true,
            })
            .collect();
        // Get definitions to end.
        let mut defs_to_end = ic_names_to_end
            .iter()
            .map(|ic_name| existing.remove(&ic_name.clone()).unwrap())
            .collect::<Vec<_>>();
        // End them and join them all.
        join_all(defs_to_end.iter_mut().map(|def| async { def.end().await })).await;
        drop(defs_to_end);
        drop(ic_names_to_end);

        // Now we add the incoming definitions.
        for definition in definitions.into_iter() {
            existing.insert(
                definition.name.clone(),
                definition.run(self.rt.clone(), metrics.clone()).await,
            );
        }
        Ok(())
    }

    /// Start a list of definitions.
    ///
    /// If replace_existing is true, any running definition matching the name
    /// of any of the incoming definitions will be stopped.  If it is false,
    /// any incoming definition named after any running definition will
    /// add an AlreadyExists error to the errors list.
    pub(crate) async fn start(
        &self,
        definitions: Vec<Definition>,
        start_mode: StartMode,
        metrics: RunningDefinitionsMetrics,
    ) -> Result<(), StartDefinitionsError> {
        let mut existing = self.definitions.lock().await;
        self.start_inner(&mut existing, definitions, start_mode, metrics).await
    }

    pub(crate) async fn definition_count(&self) -> usize {
        self.definitions.lock().await.len()
    }

    /// Stop all definitions and end.
    pub(crate) async fn end(&self) {
        let mut existing = self.definitions.lock().await;
        for (_, definition) in existing.iter_mut() {
            definition.end().await
        }
        existing.clear()
    }

    pub(crate) async fn stop(&self, definition_names: Vec<String>) -> Result<(), StopDefinitionsError> {
        let mut defs = self.definitions.lock().await;
        let mut errors: Vec<StopDefinitionError> = definition_names
            .clone()
            .into_iter()
            .filter(|n| !defs.contains_key(n))
            .map(|n| StopDefinitionError::DoesNotExist(n.clone()))
            .collect();
        errors.extend(
            definition_names
                .iter()
                .filter(|n| **n == Network::Mainnet.legacy_name() && !self.allow_mercury_deletion)
                .map(|n| StopDefinitionError::DeletionDisallowed(n.clone())),
        );
        if !errors.is_empty() {
            return Err(StopDefinitionsError { errors });
        }

        for name in definition_names.into_iter() {
            defs.remove(&name).unwrap().end().await
        }
        Ok(())
    }
}
