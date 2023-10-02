use std::{error::Error, fmt, io, path::Path};

use ic_types::PrincipalId;
use serde::Deserialize;

use crate::{
    notification::Notification,
    sink::{Sink, SinkError, WebhookSink},
};

const CONFIG_FILE_PATH_VAR_NAME: &str = "ROUTER_CONFIG_PATH";

#[derive(Debug)]
struct Route {
    matcher: Matcher,
    sinks: Vec<Sink>,
}

impl Route {
    fn matches(&self, notification: &Notification) -> bool {
        self.matcher.matches(notification)
    }
}

#[derive(Deserialize, Debug)]
struct Matcher {
    pub node_provider_id: Option<PrincipalId>,
}

impl Matcher {
    fn matches(&self, notification: &Notification) -> bool {
        self.matches_node_provider_id(notification)
    }

    fn matches_node_provider_id(&self, notification: &Notification) -> bool {
        match self.node_provider_id {
            None => true,
            Some(principal) => match &notification.node_provider {
                Some(node_provider) => principal == node_provider.principal,
                None => false,
            },
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct RouterConfig {
    #[serde(default)]
    node_providers: Vec<NPMatch>,
}

#[derive(Deserialize, Debug, Clone)]
struct NPMatch {
    principal_id: PrincipalId,
    url: url::Url,
}

impl RouterConfig {
    fn load_from_file(file_path: String) -> Result<RouterConfig, RouterConfigError> {
        let path = Path::new(&file_path);
        let contents = std::fs::read_to_string(path).map_err(RouterConfigError::File)?;
        Self::load(&contents)
    }

    fn load(contents: &str) -> Result<RouterConfig, RouterConfigError> {
        serde_yaml::from_str(contents).map_err(RouterConfigError::ConfigParsing)
    }

    fn get_routes(&self) -> Vec<Route> {
        self.node_providers
            .clone()
            .into_iter()
            .map(|np| Route {
                matcher: Matcher {
                    node_provider_id: Some(np.principal_id),
                },
                sinks: vec![Sink::Webhook(WebhookSink {
                    url: np.url,
                    auth: None,
                })],
            })
            .collect()
    }
}

#[derive(Debug)]
enum RouterConfigError {
    ConfigParsing(serde_yaml::Error),
    File(io::Error),
}

impl fmt::Display for RouterConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigParsing(_) => write!(f, "error parsing config"),
            Self::File(_) => write!(f, "error reading file"),
        }
    }
}

impl Error for RouterConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self {
            Self::ConfigParsing(e) => Some(e),
            Self::File(e) => Some(e),
        }
    }
}

#[derive(Debug)]
pub struct Router {
    routes: Vec<Route>,
}

impl Router {
    pub fn new_from_config_file() -> Result<Router, Box<dyn Error + 'static>> {
        match std::env::var(CONFIG_FILE_PATH_VAR_NAME) {
            Ok(p) => {
                let config = RouterConfig::load_from_file(p)?;
                Ok(Self::from(config))
            }
            Err(_) => Ok(Self::new()),
        }
    }

    pub fn new() -> Self {
        Self { routes: vec![] }
    }

    #[cfg(test)]
    pub fn new_from_config(contents: &str) -> Result<Router, Box<dyn std::error::Error>> {
        let config = RouterConfig::load(contents)?;
        Ok(Self::from(config))
    }

    pub async fn route(&self, notification: Notification) -> Result<(), SinkError> {
        for route in self.routes.iter() {
            if route.matches(&notification) {
                for sink in &route.sinks {
                    // TODO Ensure we try sending to all sinks even if one fail
                    // As it is, if one sink was to fail, all of the sinks after
                    // it would fail as well
                    sink.send(notification.clone()).await?
                }
            }
        }
        Ok(())
    }
}

impl From<RouterConfig> for Router {
    fn from(config: RouterConfig) -> Self {
        Self {
            routes: config.get_routes(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Route, Router, RouterConfig};

    use ic_management_types::{Provider, Status};
    use ic_types::PrincipalId;
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};
    use std::path::Path;
    use std::{fs::File, io::Write, str::FromStr, sync::Arc};

    use crate::router::CONFIG_FILE_PATH_VAR_NAME;
    use crate::{
        notification::Notification,
        sink::{Sink, TestSink},
    };

    use super::Matcher;

    #[test]
    fn loading_config() {
        let config = r#"
node_providers:
  - principal_id: eipr5-izbom-neyqh-s3ec2-52eww-cyfpg-qfomg-3dpwj-4pffh-34xcu-7qe
    url: https://localhost:8080"#;
        let temp_dir = tempfile::tempdir().unwrap();
        let cfg_file_path = temp_dir.path().join("config.yaml");
        let mut cfg_file = File::create(&cfg_file_path).unwrap();
        writeln!(cfg_file, "{}", config).unwrap();

        let router_config = RouterConfig::load_from_file(cfg_file_path.to_str().unwrap().to_string()).unwrap();
        assert!(router_config.node_providers.len() == 1);

        let routes = router_config.get_routes();
        assert_eq!(routes.len(), 1);
        let route = &routes[0];
        assert_eq!(
            route.matcher.node_provider_id,
            PrincipalId::from_str("eipr5-izbom-neyqh-s3ec2-52eww-cyfpg-qfomg-3dpwj-4pffh-34xcu-7qe").ok()
        );
        assert_eq!(route.sinks.len(), 1);
        let sink = &route.sinks[0];

        match sink {
            Sink::Webhook(s) => assert_eq!(s.url, url::Url::parse("https://localhost:8080").unwrap()),
            _ => unreachable!(),
        }
    }

    #[test]
    fn load_empty_config() {
        let config = "";
        let router = Router::new_from_config(config);
        assert!(router.is_ok());
        let router = router.unwrap();
        assert_eq!(router.routes.len(), 0);
    }

    #[test]
    fn loading_config_when_config_path_var_name_not_defined() {
        if std::env::var(CONFIG_FILE_PATH_VAR_NAME).is_err() {
            let router = Router::new_from_config_file();
            assert!(router.is_ok());
            assert_eq!(router.unwrap().routes.len(), 0);
        }
    }

    #[test]
    fn fail_loading_config_from_non_existing_file() {
        let random_file_name: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect();

        // This file has good chances not to exist
        let filepath = Path::new("/tmp/").join(random_file_name);
        std::env::set_var(CONFIG_FILE_PATH_VAR_NAME, filepath.as_os_str());
        let router = Router::new_from_config_file();
        assert!(router.is_err());
    }

    #[test]
    fn node_provider_matching() {
        let principal_id_1 = PrincipalId::new_user_test_id(1);
        let principal_id_2 = PrincipalId::new_user_test_id(2);

        let m_some_1 = Matcher {
            node_provider_id: Some(principal_id_1),
        };
        let m_some_2 = Matcher {
            node_provider_id: Some(principal_id_2),
        };
        let m_none = Matcher { node_provider_id: None };

        let notification_some_1 = Notification {
            node_id: PrincipalId::new_node_test_id(0),
            node_provider: Some(Provider {
                principal: principal_id_1,
                name: None,
                website: None,
            }),
            status_change: (Status::Healthy, Status::Degraded),
        };

        let notification_some_2 = Notification {
            node_id: PrincipalId::new_node_test_id(1),
            node_provider: Some(Provider {
                principal: principal_id_2,
                name: None,
                website: None,
            }),
            status_change: (Status::Healthy, Status::Degraded),
        };

        let notification_none = Notification {
            node_id: PrincipalId::new_node_test_id(1),
            node_provider: None,
            status_change: (Status::Healthy, Status::Degraded),
        };

        assert!(m_some_1.matches(&notification_some_1));
        assert!(!m_some_2.matches(&notification_some_1));
        assert!(m_none.matches(&notification_some_1));

        assert!(!m_some_1.matches(&notification_some_2));
        assert!(m_some_2.matches(&notification_some_2));
        assert!(m_none.matches(&notification_some_2));

        assert!(!m_some_1.matches(&notification_none));
        assert!(!m_some_2.matches(&notification_none));
        assert!(m_none.matches(&notification_none));
    }

    #[actix_web::test]
    async fn notifications_routing() {
        let principal_id_1 = PrincipalId::new_user_test_id(1);
        let notification_some_1 = Notification {
            node_id: PrincipalId::new_node_test_id(0),
            node_provider: Some(Provider {
                principal: principal_id_1,
                name: None,
                website: None,
            }),
            status_change: (Status::Healthy, Status::Degraded),
        };

        let principal_id_2 = PrincipalId::new_user_test_id(2);
        let notification_some_2 = Notification {
            node_id: PrincipalId::new_node_test_id(1),
            node_provider: Some(Provider {
                principal: principal_id_2,
                name: None,
                website: None,
            }),
            status_change: (Status::Healthy, Status::Degraded),
        };

        let test_sink = Arc::new(TestSink::new());
        let router = Router {
            routes: vec![Route {
                matcher: Matcher {
                    node_provider_id: Some(principal_id_1),
                },
                sinks: vec![Sink::Test(test_sink.clone())],
            }],
        };

        let _ = router.route(notification_some_1.clone()).await;
        let _ = router.route(notification_some_2.clone()).await;

        let received_notifications = test_sink.notifications();
        assert_eq!(received_notifications.len(), 1);
        assert_eq!(received_notifications[0], notification_some_1);
    }
}
