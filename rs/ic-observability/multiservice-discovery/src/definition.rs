use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use ic_registry_client::client::ThresholdSigPublicKey;
use service_discovery::job_types::JobType;
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
pub(crate) struct RunningDefinition {
    pub(crate) definition: Definition,
    stop_signal: Receiver<()>,
    ender: Arc<Mutex<Option<Ender>>>,
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

    pub(crate) async fn run(self, rt: tokio::runtime::Handle) -> RunningDefinition {
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

    async fn initial_registry_sync(&self) {
        info!(
            self.definition.log,
            "Syncing local registry for {} started", self.definition.name
        );
        info!(
            self.definition.log,
            "Using local registry path: {}",
            self.definition.registry_path.display()
        );

        sync_local_registry(
            self.definition.log.clone(),
            self.definition.registry_path.join("targets"),
            self.definition.nns_urls.clone(),
            false,
            self.definition.public_key,
        )
        .await;

        info!(
            self.definition.log,
            "Syncing local registry for {} completed", self.definition.name
        );
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
            }
            debug!(self.definition.log, "Update registries for {}", self.definition.name);
            if let Err(e) = self.definition.ic_discovery.update_registries().await {
                warn!(
                    self.definition.log,
                    "Failed to sync registry for {} @ interval {:?}: {:?}", self.definition.name, tick, e
                );
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

    async fn run(&self) {
        self.initial_registry_sync().await;

        info!(
            self.definition.log,
            "Starting to watch for changes for definition {}", self.definition.name
        );

        self.poll_loop().await;

        if self.definition.name != "mercury" {
            info!(
                self.definition.log,
                "Removing registry dir '{}' for definition {}...",
                self.definition.registry_path.display(),
                self.definition.name
            );

            if let Err(e) = std::fs::remove_dir_all(self.definition.registry_path.clone()) {
                warn!(
                    self.definition.log,
                    "Failed to remove registry dir for definition {}: {:?}", self.definition.name, e
                );
            }
        }
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
}

impl Error for StartDefinitionError {}

impl Display for StartDefinitionError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        match self {
            Self::AlreadyExists(name) => write!(f, "definition {} is already running", name),
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
}

impl Error for StopDefinitionError {}

impl Display for StopDefinitionError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        match self {
            Self::DoesNotExist(name) => write!(f, "definition {} does not exist", name),
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

#[derive(Clone)]
pub(crate) struct DefinitionsSupervisor {
    rt: tokio::runtime::Handle,
    pub(crate) definitions: Arc<Mutex<BTreeMap<String, RunningDefinition>>>,
}

impl DefinitionsSupervisor {
    pub(crate) fn new(rt: tokio::runtime::Handle) -> Self {
        DefinitionsSupervisor {
            rt,
            definitions: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    async fn start_inner(
        &self,
        existing: &mut BTreeMap<String, RunningDefinition>,
        definitions: Vec<Definition>,
        replace_existing: bool,
    ) -> Result<(), StartDefinitionsError> {
        let mut error = StartDefinitionsError { errors: vec![] };
        let mut names_added: HashSet<String> = HashSet::new();

        for definition in definitions.iter() {
            let dname = definition.name.clone();
            // Check if we already have something running with the same name,
            // if the user does not want to replace those with newer defs.
            if !replace_existing && existing.contains_key(&dname) {
                error.errors.push(StartDefinitionError::AlreadyExists(dname.clone()));
                continue;
            }

            // Check for incoming duplicates.
            if names_added.contains(&dname) {
                error.errors.push(StartDefinitionError::AlreadyExists(dname.clone()));
                continue;
            }
            names_added.insert(dname);
        }

        if !error.errors.is_empty() {
            return Err(error);
        }

        for definition in definitions.into_iter() {
            // Here is where we stop already running definitions
            // that have a name similar to the one being added.
            if let Some(d) = existing.get_mut(&definition.name) {
                d.end().await
            }
            // We stop X before we start X' because otherwise
            // the newly-running definition will fight over
            // shared disk space (a folder) and probably die.
            existing.insert(definition.name.clone(), definition.run(self.rt.clone()).await);
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
        replace_existing: bool,
    ) -> Result<(), StartDefinitionsError> {
        let mut u = self.definitions.lock().await;
        self.start_inner(&mut u, definitions, replace_existing).await
    }

    async fn end_inner(&self, definitions: &mut BTreeMap<String, RunningDefinition>) {
        for (_, definition) in definitions.iter_mut() {
            definition.end().await
        }
        definitions.clear()
    }

    pub(crate) async fn end(&self) {
        let mut u = self.definitions.lock().await;
        self.end_inner(&mut u).await
    }

    pub(crate) async fn stop(&self, definition_names: Vec<String>) -> Result<(), StopDefinitionsError> {
        let mut defs = self.definitions.lock().await;
        let errors: Vec<StopDefinitionError> = definition_names
            .iter()
            .filter(|n| defs.contains_key(*n))
            .map(|n| StopDefinitionError::DoesNotExist(n.clone()))
            .collect();
        if !errors.is_empty() {
            return Err(StopDefinitionsError { errors });
        }

        for name in definition_names.into_iter() {
            defs.remove(&name).unwrap().end().await
        }
        Ok(())
    }
}
