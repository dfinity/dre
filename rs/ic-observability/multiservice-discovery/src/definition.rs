use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use futures_util::future::join_all;
use ic_registry_client::client::ThresholdSigPublicKey;
use multiservice_discovery_shared::contracts::target::map_to_target_dto;
use multiservice_discovery_shared::contracts::target::TargetDto;
use serde::Deserialize;
use serde::Serialize;
use service_discovery::job_types::{JobType, NodeOS};
use service_discovery::registry_sync::SyncError;
use service_discovery::IcServiceDiscovery;
use service_discovery::IcServiceDiscoveryError;
use service_discovery::TargetGroup;
use service_discovery::{registry_sync::sync_local_registry, IcServiceDiscoveryImpl};
use slog::error;
use slog::{debug, info, warn, Logger};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::Debug;
use std::fmt::{Display, Error as FmtError, Formatter};
use std::fs;
use std::io::Write;
use std::net::SocketAddr;
use std::sync::Arc;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use url::Url;

use crate::make_logger;
use crate::metrics::RunningDefinitionsMetrics;

#[derive(Clone, Serialize, Deserialize)]
pub struct FSDefinition {
    pub nns_urls: Vec<Url>,
    pub registry_path: PathBuf,
    pub name: String,
    pub public_key: Option<ThresholdSigPublicKey>,
    pub poll_interval: Duration,
    pub registry_query_timeout: Duration,
    pub boundary_nodes: Vec<BoundaryNode>,
}

impl From<Definition> for FSDefinition {
    fn from(definition: Definition) -> Self {
        Self {
            nns_urls: definition.nns_urls,
            registry_path: definition.registry_path,
            name: definition.name,
            public_key: definition.public_key,
            poll_interval: definition.poll_interval,
            registry_query_timeout: definition.registry_query_timeout,
            boundary_nodes: definition.boundary_nodes,
        }
    }
}

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

impl PartialEq for Definition {
    fn eq(&self, other: &Self) -> bool {
        self.nns_urls == other.nns_urls
            && self.registry_path == other.registry_path
            && self.name == other.name
            && self.public_key == other.public_key
            && self.poll_interval == other.poll_interval
            && self.registry_query_timeout == other.registry_query_timeout
            && self.boundary_nodes == other.boundary_nodes
    }
}

impl From<FSDefinition> for Definition {
    fn from(fs_definition: FSDefinition) -> Self {
        if std::fs::metadata(&fs_definition.registry_path).is_err() {
            std::fs::create_dir_all(fs_definition.registry_path.clone()).unwrap();
        }
        let log = make_logger();
        Self {
            nns_urls: fs_definition.nns_urls,
            registry_path: fs_definition.registry_path.clone(),
            name: fs_definition.name,
            log: log.clone(),
            public_key: fs_definition.public_key,
            poll_interval: fs_definition.poll_interval,
            registry_query_timeout: fs_definition.registry_query_timeout,
            ic_discovery: Arc::new(IcServiceDiscoveryImpl::new(log, fs_definition.registry_path, fs_definition.registry_query_timeout).unwrap()),
            boundary_nodes: fs_definition.boundary_nodes,
        }
    }
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
    pub async fn sync_and_stop(&self, skip_update_local_registry: bool) {
        // If skip_update_local_registry is true, first try and use the existing one
        if skip_update_local_registry {
            match self.running_def.initial_registry_sync(true).await {
                Ok(()) => return,
                Err(e) => {
                    error!(
                        self.running_def.definition.log,
                        "Error while running initial sync with the registry for definition named '{}': {:?}", self.running_def.definition.name, e
                    );
                    self.running_def.metrics.observe_sync(self.running_def.name(), false);
                }
            }
        }
        // If skip_update_local_registry is false, or the inital sync failed try to do a full initial sync
        if let Err(e) = self.running_def.initial_registry_sync(false).await {
            error!(
                self.running_def.definition.log,
                "Error while running full initial sync with the registry for definition named '{}': {:?}", self.running_def.definition.name, e
            );
            self.running_def.metrics.observe_sync(self.running_def.name(), false);
        }
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
    pub(crate) async fn end(&self) {
        let mut ender = self.ender.lock().await;
        if let Some(s) = ender.take() {
            // We have pulled out the channel from its container.  After this,
            // all senders will have been dropped, and no more messages can be sent.
            // https://docs.rs/crossbeam/latest/crossbeam/channel/index.html#disconnection
            info!(self.definition.log, "Sending termination signal to definition {}", self.definition.name);
            s.stop_signal_sender.send(()).unwrap();
            info!(self.definition.log, "Joining definition {} thread", self.definition.name);
            s.join_handle.join().unwrap();
        }
    }

    pub(crate) fn get_target_groups(&self, job_type: JobType) -> Result<BTreeSet<TargetGroup>, IcServiceDiscoveryError> {
        self.definition.ic_discovery.get_target_groups(job_type, self.definition.log.clone())
    }

    async fn initial_registry_sync(&self, use_current_version: bool) -> Result<(), SyncError> {
        info!(
            self.definition.log,
            "Syncing local registry for {} (to local registry path {}) started",
            self.definition.name,
            self.definition.registry_path.display(),
        );

        let r = sync_local_registry(
            self.definition.log.clone(),
            self.definition.registry_path.join("targets"),
            &self.definition.nns_urls.clone(),
            use_current_version,
            self.definition.public_key,
            &self.stop_signal,
        )
        .await;
        match r {
            Ok(_) => {
                info!(self.definition.log, "Syncing local registry for {} completed", self.definition.name,);
                self.metrics.observe_sync(self.name(), true);
                Ok(())
            }
            Err(e) => {
                match e {
                    SyncError::PublicKey(ref pkey) => {
                        error!(self.definition.log, "Failure in initial sync of {}: {}", self.definition.name, pkey,);
                        // Note failure in metrics.  On the other leg of the match
                        // we do not note either success or failure, since we don't
                        // know yet whether it was successful or not.
                        self.metrics.observe_sync(self.name(), false);
                    }
                    SyncError::Interrupted => info!(self.definition.log, "Interrupted initial sync of {}", self.definition.name),
                };
                Err(e)
            }
        }
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
                self.metrics.observe_load(self.name(), false)
            } else {
                self.metrics.observe_load(self.name(), true)
            }
            debug!(self.definition.log, "Update registries for {}", self.definition.name);
            if let Err(e) = self.definition.ic_discovery.update_registries().await {
                warn!(
                    self.definition.log,
                    "Failed to sync registry for {} @ interval {:?}: {:?}", self.definition.name, tick, e
                );
                self.metrics.observe_sync(self.name(), false)
            } else {
                self.metrics.observe_sync(self.name(), true)
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
        // Loop to do retries of initial sync and handle cancellation.
        // We keep retries outside the callee to make the callee easier
        // to test and more solid state.
        while let Err(e) = self.initial_registry_sync(false).await {
            match e {
                SyncError::Interrupted => {
                    // Signal sent to callee via channel, initial sync interrupted.
                    // We signal observation end because we are going to return.
                    self.metrics.observe_end(self.name());
                    return;
                }
                SyncError::PublicKey(_) => {
                    // Initial sync failed.
                    error!(
                        self.definition.log,
                        "Will retry sync of {} until successful after {:#?}", self.definition.name, self.definition.poll_interval,
                    );
                    // Wait a prudent interval before retrying, but watch for
                    // termination during that wait.
                    let interval = crossbeam::channel::tick(self.definition.poll_interval);
                    crossbeam::select! {
                        recv(self.stop_signal) -> _ => {
                            // Terminated!  Note the event and mark sync end.
                            info!(self.definition.log, "Received shutdown signal while waiting for initial sync retry of definition {}", self.definition.name);
                            self.metrics.observe_end(self.name());
                            return;
                        },
                        recv(interval) -> _ => continue,
                    }
                }
            }
        }

        // Ready to incrementally sync.
        info!(
            self.definition.log,
            "Starting to watch for changes for definition {}", self.definition.name
        );

        self.poll_loop().await;

        self.metrics.observe_end(self.name());
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

#[derive(Clone, Serialize, Deserialize, PartialEq)]
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
    networks_state_file: Option<PathBuf>,
    log: Logger,
}

impl DefinitionsSupervisor {
    pub(crate) fn new(rt: tokio::runtime::Handle, allow_mercury_deletion: bool, networks_state_file: Option<PathBuf>, log: Logger) -> Self {
        DefinitionsSupervisor {
            rt,
            definitions: Arc::new(Mutex::new(BTreeMap::new())),
            allow_mercury_deletion,
            networks_state_file,
            log,
        }
    }

    pub(crate) async fn load_or_create_defs(&self, metrics: RunningDefinitionsMetrics) -> Result<(), Box<dyn Error>> {
        if let Some(networks_state_file) = self.networks_state_file.clone() {
            if networks_state_file.exists() {
                let file_content = fs::read_to_string(networks_state_file.clone())?;
                let initial_definitions: Vec<FSDefinition> = serde_json::from_str(&file_content)?;
                let names = initial_definitions.iter().map(|def| def.name.clone()).collect::<Vec<_>>();
                info!(self.log, "Definitions loaded from {:?}:\n{:?}", networks_state_file.as_path(), names);
                self.start(
                    initial_definitions.into_iter().map(|def| def.into()).collect(),
                    StartMode::AddToDefinitions,
                    metrics,
                )
                .await?;
            }
        }
        Ok(())
    }

    // FIXME: if the file contents on disk are the same as the contents about to
    // be persisted, then the file should not be overwritten because it was
    // already updated by another MSD sharing the same directory.
    pub(crate) async fn persist_defs(&self, existing: &mut BTreeMap<String, RunningDefinition>) -> Result<(), Box<dyn Error>> {
        if let Some(networks_state_file) = self.networks_state_file.clone() {
            retry::retry(retry::delay::Exponential::from_millis(10).take(5), || {
                std::fs::OpenOptions::new()
                    .create(true)
                    .truncate(true)
                    .write(true)
                    .open(networks_state_file.as_path())
                    .and_then(|mut file| {
                        let fs_def: Vec<FSDefinition> = existing
                            .values()
                            .cloned()
                            .map(|running_def| running_def.definition.into())
                            .collect::<Vec<_>>();

                        file.write_all(serde_json::to_string(&fs_def)?.as_bytes()).map(|_| file)
                    })
                    .and_then(|mut file| file.flush())
            })?;
        }
        Ok(())
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

        if !self.allow_mercury_deletion && !ic_names_to_add.contains("mercury") && start_mode == StartMode::ReplaceExistingDefinitions {
            error.errors.push(StartDefinitionError::DeletionDisallowed("mercury".to_string()))
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
            existing.insert(definition.name.clone(), definition.run(self.rt.clone(), metrics.clone()).await);
        }
        // Now we rewrite definitions to disk.
        if let Err(e) = self.persist_defs(existing).await {
            warn!(self.log, "Error while peristing definitions to disk '{}'", e);
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
                .filter(|n| *n == "mercury" && !self.allow_mercury_deletion)
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

#[derive(Deserialize)]
pub struct TargetFilterSpec {
    pub node_provider_id: Option<String>,
    pub operator_id: Option<String>,
    pub dc_id: Option<String>,
    pub ic_name: Option<String>,
    pub subnet_id: Option<String>,
}

impl TargetFilterSpec {
    pub fn matches_ic_node(&self, t: &TargetDto) -> bool {
        // self.ic_name is explicitly excluded here.
        // Call self.matches_ic().
        let o = match &self.operator_id {
            None => true,
            Some(operator_id) => t.operator_id.to_string() == *operator_id,
        };
        let n = match &self.node_provider_id {
            None => true,
            Some(node_provider_id) => t.node_provider_id.to_string() == *node_provider_id,
        };
        let d = match &self.dc_id {
            None => true,
            Some(dc_id) => *t.dc_id == *dc_id,
        };
        let s = match &self.subnet_id {
            None => true,
            Some(subnet_id) => match t.subnet_id {
                Some(t_subnet_id) => t_subnet_id.to_string() == *subnet_id,
                None => subnet_id.as_str() == "",
            },
        };
        o && n && d && s
    }

    pub fn matches_boundary_node(&self, b: &BoundaryNode) -> bool {
        // self.ic_name is explicitly excluded here.
        // Call self.matches_ic().
        if self.operator_id.is_some() || self.node_provider_id.is_some() || self.subnet_id.is_some() {
            return false;
        };
        let d = match &self.dc_id {
            None => true,
            Some(dc_id) => match b.custom_labels.get("dc") {
                Some(b_dc_id) => *b_dc_id == *dc_id,
                None => "" == dc_id.as_str(),
            },
        };
        d
    }

    pub fn matches_ic(&self, ic_name: &String) -> bool {
        match &self.ic_name {
            None => true,
            Some(my_ic_name) => *ic_name == *my_ic_name,
        }
    }

    pub fn empty() -> Self {
        Self {
            node_provider_id: None,
            operator_id: None,
            dc_id: None,
            ic_name: None,
            subnet_id: None,
        }
    }
}

pub fn ic_node_target_dtos_from_definitions(definitions: &BTreeMap<String, RunningDefinition>, filters: &TargetFilterSpec) -> Vec<TargetDto> {
    from_definitions_into_targets(definitions, filters, JobType::all_for_ic_nodes(), |target| target.is_api_bn)
}

pub fn api_boundary_nodes_target_dtos_from_definitions(
    definitions: &BTreeMap<String, RunningDefinition>,
    filters: &TargetFilterSpec,
) -> Vec<TargetDto> {
    from_definitions_into_targets(definitions, filters, JobType::all_for_api_boundary_nodes(), |target| !target.is_api_bn)
}

fn from_definitions_into_targets(
    definitions: &BTreeMap<String, RunningDefinition>,
    filters: &TargetFilterSpec,
    jobs: Vec<JobType>,
    skip_filter: impl Fn(&TargetGroup) -> bool,
) -> Vec<TargetDto> {
    let mut result: Vec<TargetDto> = vec![];

    for (_, def) in definitions.iter() {
        if filters.matches_ic(&def.name()) {
            for job_type in &jobs {
                let target_groups = match def.get_target_groups(*job_type) {
                    Ok(target_groups) => target_groups,
                    Err(_) => continue,
                };

                target_groups.iter().for_each(|target_group| {
                    if skip_filter(target_group) {
                        return;
                    }
                    if let Some(target) = result.iter_mut().find(|t| t.node_id == target_group.node_id) {
                        target.jobs.push(*job_type);
                    } else {
                        let target = map_to_target_dto(target_group, *job_type, BTreeMap::new(), target_group.node_id.to_string(), def.name());
                        if filters.matches_ic_node(&target) {
                            result.push(target)
                        };
                    }
                });
            }
        }
    }

    result
}

pub fn boundary_nodes_from_definitions(definitions: &BTreeMap<String, RunningDefinition>, filters: &TargetFilterSpec) -> Vec<(String, BoundaryNode)> {
    definitions
        .iter()
        .filter(|(_, def)| filters.matches_ic(&def.name()))
        .flat_map(|(_, def)| {
            def.definition.boundary_nodes.iter().filter_map(|bn| {
                // Since boundary nodes have been checked for correct job
                // type when they were added via POST, then we can trust
                // the correct job type is at play here.
                // If, however, this boundary node is under the test environment,
                // and the job is Node Exporter, then skip adding this
                // target altogether.
                if bn.custom_labels.iter().any(|(k, v)| k.as_str() == "env" && v.as_str() == "test")
                    && bn.job_type == JobType::NodeExporter(NodeOS::Host)
                {
                    return None;
                }
                if !filters.matches_boundary_node(bn) {
                    return None;
                }
                Some((def.name(), bn.clone()))
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Definition, TestDefinition};
    use crate::{definition::DefinitionsSupervisor, make_logger, metrics::RunningDefinitionsMetrics};
    use ic_management_types::Network;
    use std::{collections::BTreeMap, str::FromStr, time::Duration};
    use tempfile::tempdir;

    #[tokio::test]
    async fn persist_defs() {
        let handle = tokio::runtime::Handle::current();
        let definitions_dir = tempdir().unwrap();
        let definitions_path = definitions_dir.path().join(String::from("definitions.json"));
        let log = make_logger();
        let supervisor = DefinitionsSupervisor::new(handle.clone(), false, Some(definitions_path.clone()), log.clone());

        let mocked_definition = Definition::new(
            vec![url::Url::from_str("http://[2a00:fb01:400:42:5000:3cff:fe45:6c61]:8080").unwrap()],
            definitions_dir.as_ref().to_path_buf(),
            Network::new("mainnet", &[]).await.unwrap().legacy_name(),
            log,
            None,
            Duration::from_secs(0),
            Duration::from_secs(0),
        );
        supervisor
            .persist_defs(&mut BTreeMap::from([(
                String::from("test"),
                TestDefinition::new(mocked_definition.clone(), RunningDefinitionsMetrics::new()).running_def,
            )]))
            .await
            .unwrap();
        supervisor.definitions.lock().await.clear();
        supervisor.load_or_create_defs(RunningDefinitionsMetrics::new()).await.unwrap();
        let loaded_definition = supervisor
            .definitions
            .lock()
            .await
            .values()
            .cloned()
            .map(|def| def.definition)
            .next()
            .unwrap();

        assert_eq!(mocked_definition, loaded_definition);
    }
}
