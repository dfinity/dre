use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::{fmt, str::FromStr};

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NodeOS {
    Guest,
    Host,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, PartialOrd, Ord, Serialize, Deserialize)]
pub enum JobType {
    Replica,
    NodeExporter(NodeOS),
    Orchestrator,
    MetricsProxy(NodeOS),
}

/// By convention, the first two bytes of the host-part of the replica's IP
/// address are 0x6801. The corresponding segment for the host is 0x6800.
///
/// (The MAC starts with 0x6a00. The 7'th bit of the first byte is flipped. See
/// https://en.wikipedia.org/wiki/MAC_address)
fn guest_to_host_address(sockaddr: SocketAddr) -> SocketAddr {
    match sockaddr.ip() {
        IpAddr::V6(a) if a.segments()[4] == 0x6801 => {
            let s = a.segments();
            let new_addr = Ipv6Addr::new(s[0], s[1], s[2], s[3], 0x6800, s[5], s[6], s[7]);
            let ip = IpAddr::V6(new_addr);
            SocketAddr::new(ip, sockaddr.port())
        }
        _ip => sockaddr,
    }
}

// The type of discovered job.
impl JobType {
    pub fn port(&self) -> u16 {
        match self {
            Self::Replica => 9090,
            Self::NodeExporter(NodeOS::Host) => 9100,
            Self::NodeExporter(NodeOS::Guest) => 9100,
            Self::Orchestrator => 9091,
            Self::MetricsProxy(NodeOS::Host) => 19100,
            Self::MetricsProxy(NodeOS::Guest) => 19100,
        }
    }
    pub fn endpoint(&self) -> &'static str {
        match self {
            Self::Replica => "/",
            Self::NodeExporter(_) => "/metrics",
            Self::Orchestrator => "/",
            Self::MetricsProxy(_) => "/metrics",
        }
    }
    pub fn scheme(&self) -> &'static str {
        match self {
            Self::Replica => "http",
            Self::NodeExporter(_) => "https",
            Self::Orchestrator => "http",
            Self::MetricsProxy(_) => "https",
        }
    }

    // Return the socket address with the correct port and IP address.
    // Any non-guest IP address is returned unchanged.  Any guest IP
    // address that needs changing to host is returned with host IP.
    // Boundary nodes are correctly handled.
    // FIXME: make me private!
    pub fn sockaddr(&self, s: SocketAddr, is_boundary_node: bool) -> SocketAddr {
        let mut ss = s;
        ss.set_port(self.port());
        if *self == Self::NodeExporter(NodeOS::Host) {
            guest_to_host_address(ss)
        } else if *self == Self::MetricsProxy(NodeOS::Host) {
            match is_boundary_node {
                // This is a boundary node IP.  Return it unchanged.
                true => ss,
                // Change GuestOS IP to HostOS IP.
                false => guest_to_host_address(ss),
            }
        } else {
            ss
        }
    }

    pub fn ip(&self, s: SocketAddr, is_boundary_node: bool) -> IpAddr {
        self.sockaddr(s, is_boundary_node).ip()
    }

    pub fn url(&self, s: SocketAddr, is_boundary_node: bool) -> String {
        format!(
            "{}://{}/{}",
            self.scheme(),
            self.sockaddr(s, is_boundary_node),
            self.endpoint().trim_start_matches('/'),
        )
    }
}

/// This is duplicated in impl Job.
impl JobType {
    pub fn all_for_ic_nodes() -> Vec<Self> {
        [
            JobType::Replica,
            JobType::Orchestrator,
            JobType::NodeExporter(NodeOS::Guest),
            JobType::NodeExporter(NodeOS::Host),
            JobType::MetricsProxy(NodeOS::Host),
            JobType::MetricsProxy(NodeOS::Guest),
        ]
        .into_iter()
        .collect::<Vec<Self>>()
    }

    pub fn all_for_boundary_nodes() -> Vec<Self> {
        [
            JobType::NodeExporter(NodeOS::Guest),
            JobType::NodeExporter(NodeOS::Host),
        ]
        .into_iter()
        .collect::<Vec<Self>>()
    }

    pub fn all_for_logs() -> Vec<Self> {
        [
            JobType::NodeExporter(NodeOS::Guest),
            JobType::NodeExporter(NodeOS::Host),
        ]
        .into_iter()
        .collect::<Vec<Self>>()
    }
}

#[derive(Debug)]
pub struct JobTypeParseError {
    input: String,
}
impl std::error::Error for JobTypeParseError {}

impl fmt::Display for JobTypeParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Could not parse {} into a job type", self.input)
    }
}

impl FromStr for JobType {
    type Err = JobTypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            // When a new job type is added, please do not forget to
            // update its antipode method at fmt() below.
            "replica" => Ok(JobType::Replica),
            "node_exporter" => Ok(JobType::NodeExporter(NodeOS::Guest)),
            "host_node_exporter" => Ok(JobType::NodeExporter(NodeOS::Host)),
            "orchestrator" => Ok(JobType::Orchestrator),
            "host_metrics_proxy" => Ok(JobType::MetricsProxy(NodeOS::Host)),
            "guest_metrics_proxy" => Ok(JobType::MetricsProxy(NodeOS::Guest)),
            _ => Err(JobTypeParseError { input: s.to_string() }),
        }
    }
}

impl From<String> for JobType {
    fn from(value: String) -> Self {
        match JobType::from_str(&value) {
            Ok(val) => val,
            Err(_) => panic!("Couldn't parse JobType"),
        }
    }
}

impl fmt::Display for JobType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // When a new job type is added, please do not forget to
            // update its antipode method at from_str() above.
            JobType::Replica => write!(f, "replica"),
            JobType::NodeExporter(NodeOS::Guest) => write!(f, "node_exporter"),
            JobType::NodeExporter(NodeOS::Host) => write!(f, "host_node_exporter"),
            JobType::Orchestrator => write!(f, "orchestrator"),
            JobType::MetricsProxy(NodeOS::Host) => write!(f, "host_metrics_proxy"),
            JobType::MetricsProxy(NodeOS::Guest) => write!(f, "guest_metrics_proxy"),
        }
    }
}
