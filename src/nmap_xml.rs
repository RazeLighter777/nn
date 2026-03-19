use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NmapRunXml {
    pub scanner: Option<String>,
    pub args: Option<String>,
    pub start: Option<i64>,
    pub startstr: Option<String>,
    pub version: Option<String>,
    pub profile_name: Option<String>,
    pub xmloutputversion: Option<String>,
    #[serde(rename = "scaninfo", default)]
    pub scaninfo: Vec<ScanInfoXml>,
    pub verbose: Option<LevelXml>,
    pub debugging: Option<LevelXml>,
    #[serde(rename = "target", default)]
    pub targets: Vec<TargetXml>,
    #[serde(rename = "taskbegin", default)]
    pub taskbegin: Vec<TaskBeginXml>,
    #[serde(rename = "taskprogress", default)]
    pub taskprogress: Vec<TaskProgressXml>,
    #[serde(rename = "taskend", default)]
    pub taskend: Vec<TaskEndXml>,
    #[serde(rename = "hosthint", default)]
    pub hosthints: Vec<HostHintXml>,
    #[serde(rename = "host", default)]
    pub hosts: Vec<HostXml>,
    pub prescript: Option<ScriptContainerXml>,
    pub postscript: Option<ScriptContainerXml>,
    pub runstats: Option<RunStatsXml>,
    #[serde(rename = "output", default)]
    pub output: Vec<OutputXml>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LevelXml {
    pub level: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ScanInfoXml {
    #[serde(rename = "type")]
    pub scan_type: Option<String>,
    pub scanflags: Option<String>,
    pub protocol: Option<String>,
    pub numservices: Option<i32>,
    pub services: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TargetXml {
    pub specification: String,
    pub status: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TaskBeginXml {
    pub task: String,
    pub time: i64,
    pub extrainfo: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TaskProgressXml {
    pub task: String,
    pub time: i64,
    pub percent: String,
    pub remaining: Option<i64>,
    pub etc: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TaskEndXml {
    pub task: String,
    pub time: i64,
    pub extrainfo: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct HostHintXml {
    pub status: Option<StatusXml>,
    #[serde(rename = "address", default)]
    pub addresses: Vec<AddressXml>,
    pub hostnames: Option<HostnamesXml>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct HostXml {
    #[serde(rename = "@starttime")]
    pub starttime: Option<i64>,
    #[serde(rename = "@endtime")]
    pub endtime: Option<i64>,
    #[serde(rename = "@timedout")]
    pub timedout: Option<bool>,
    #[serde(rename = "@comment")]
    pub comment: Option<String>,
    pub status: Option<StatusXml>,
    #[serde(rename = "address", default)]
    pub addresses: Vec<AddressXml>,
    pub hostnames: Option<HostnamesXml>,
    pub ports: Option<PortsXml>,
    pub os: Option<OsXml>,
    pub smurf: Option<SmurfXml>,
    pub distance: Option<DistanceXml>,
    pub uptime: Option<UptimeXml>,
    pub tcpsequence: Option<TcpSequenceXml>,
    pub ipidsequence: Option<IpidSequenceXml>,
    pub tcptssequence: Option<TcptsSequenceXml>,
    pub trace: Option<TraceXml>,
    pub times: Option<TimesXml>,
    pub hostscript: Option<ScriptContainerXml>,
}

impl HostXml {
    pub fn ip_addresses(&self) -> Vec<String> {
        self.addresses
            .iter()
            .filter(|address| address.is_ip_address())
            .map(|address| address.addr.clone())
            .collect()
    }

    pub fn mac_address(&self) -> Option<String> {
        self.addresses
            .iter()
            .find(|address| address.is_mac_address())
            .map(|address| address.addr.clone())
    }

    pub fn hostname_values(&self) -> Vec<String> {
        self.hostnames
            .as_ref()
            .map(|hostnames| {
                hostnames
                    .hostname
                    .iter()
                    .filter_map(|hostname| hostname.name.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn display_name(&self) -> String {
        self.hostname_values()
            .into_iter()
            .next()
            .or_else(|| self.ip_addresses().into_iter().next())
            .or_else(|| self.mac_address())
            .unwrap_or_else(|| "unnamed-host".to_string())
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct StatusXml {
    #[serde(rename = "@state")]
    pub state: Option<String>,
    #[serde(rename = "@reason")]
    pub reason: Option<String>,
    #[serde(rename = "@reason_ttl")]
    pub reason_ttl: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AddressXml {
    #[serde(rename = "@addr")]
    pub addr: String,
    #[serde(rename = "@addrtype")]
    pub addrtype: Option<String>,
    #[serde(rename = "@vendor")]
    pub vendor: Option<String>,
}

impl AddressXml {
    pub fn is_ip_address(&self) -> bool {
        matches!(self.addrtype.as_deref(), Some("ipv4") | Some("ipv6"))
    }

    pub fn is_mac_address(&self) -> bool {
        matches!(self.addrtype.as_deref(), Some("mac"))
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct HostnamesXml {
    #[serde(rename = "hostname", default)]
    pub hostname: Vec<HostnameXml>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct HostnameXml {
    #[serde(rename = "@name")]
    pub name: Option<String>,
    #[serde(rename = "@type")]
    pub hostname_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PortsXml {
    #[serde(rename = "extraports", default)]
    pub extraports: Vec<ExtraPortsXml>,
    #[serde(rename = "port", default)]
    pub port: Vec<PortXml>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ExtraPortsXml {
    #[serde(rename = "@state")]
    pub state: String,
    #[serde(rename = "@count")]
    pub count: i32,
    #[serde(rename = "extrareasons", default)]
    pub extrareasons: Vec<ExtraReasonsXml>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ExtraReasonsXml {
    #[serde(rename = "@reason")]
    pub reason: String,
    #[serde(rename = "@count")]
    pub count: String,
    #[serde(rename = "@proto")]
    pub proto: Option<String>,
    #[serde(rename = "@ports")]
    pub ports: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PortXml {
    #[serde(rename = "@protocol")]
    pub protocol: String,
    #[serde(rename = "@portid")]
    pub portid: u16,
    pub state: Option<PortStateXml>,
    pub owner: Option<OwnerXml>,
    pub service: Option<ServiceXml>,
    #[serde(rename = "script", default)]
    pub scripts: Vec<ScriptXml>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PortStateXml {
    #[serde(rename = "@state")]
    pub state: Option<String>,
    #[serde(rename = "@reason")]
    pub reason: Option<String>,
    #[serde(rename = "@reason_ttl")]
    pub reason_ttl: Option<String>,
    #[serde(rename = "@reason_ip")]
    pub reason_ip: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OwnerXml {
    #[serde(rename = "@name")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ServiceXml {
    #[serde(rename = "@name")]
    pub name: Option<String>,
    #[serde(rename = "@conf")]
    pub conf: Option<i32>,
    #[serde(rename = "@method")]
    pub method: Option<String>,
    #[serde(rename = "@version")]
    pub version: Option<String>,
    #[serde(rename = "@product")]
    pub product: Option<String>,
    #[serde(rename = "@extrainfo")]
    pub extrainfo: Option<String>,
    #[serde(rename = "@tunnel")]
    pub tunnel: Option<String>,
    #[serde(rename = "@proto")]
    pub proto: Option<String>,
    #[serde(rename = "@rpcnum")]
    pub rpcnum: Option<i32>,
    #[serde(rename = "@lowver")]
    pub lowver: Option<i32>,
    #[serde(rename = "@highver")]
    pub highver: Option<i32>,
    #[serde(rename = "@hostname")]
    pub hostname: Option<String>,
    #[serde(rename = "@ostype")]
    pub ostype: Option<String>,
    #[serde(rename = "@devicetype")]
    pub devicetype: Option<String>,
    #[serde(rename = "@servicefp")]
    pub servicefp: Option<String>,
    #[serde(rename = "cpe", default)]
    pub cpe: Vec<TextNodeXml>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TextNodeXml {
    #[serde(rename = "$text")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ScriptContainerXml {
    #[serde(rename = "script", default)]
    pub scripts: Vec<ScriptXml>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ScriptXml {
    #[serde(rename = "@id")]
    pub id: Option<String>,
    #[serde(rename = "@output")]
    pub output: Option<String>,
    #[serde(rename = "table", default)]
    pub tables: Vec<ScriptTableXml>,
    #[serde(rename = "elem", default)]
    pub elems: Vec<ScriptElemXml>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ScriptTableXml {
    #[serde(rename = "@key")]
    pub key: Option<String>,
    #[serde(rename = "table", default)]
    pub tables: Vec<ScriptTableXml>,
    #[serde(rename = "elem", default)]
    pub elems: Vec<ScriptElemXml>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ScriptElemXml {
    #[serde(rename = "@key")]
    pub key: Option<String>,
    #[serde(rename = "$text")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OsXml {
    #[serde(rename = "portused", default)]
    pub portused: Vec<OsPortUsedXml>,
    #[serde(rename = "osclass", default)]
    pub osclass: Vec<OsClassXml>,
    #[serde(rename = "osmatch", default)]
    pub osmatch: Vec<OsMatchXml>,
    #[serde(rename = "osfingerprint", default)]
    pub osfingerprint: Vec<OsFingerprintXml>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OsPortUsedXml {
    #[serde(rename = "@state")]
    pub state: String,
    #[serde(rename = "@proto")]
    pub proto: String,
    #[serde(rename = "@portid")]
    pub portid: i32,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OsClassXml {
    #[serde(rename = "@vendor")]
    pub vendor: String,
    #[serde(rename = "@osgen")]
    pub osgen: Option<String>,
    #[serde(rename = "@type")]
    pub os_type: Option<String>,
    #[serde(rename = "@accuracy")]
    pub accuracy: i32,
    #[serde(rename = "@osfamily")]
    pub osfamily: String,
    #[serde(rename = "cpe", default)]
    pub cpe: Vec<TextNodeXml>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OsMatchXml {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@accuracy")]
    pub accuracy: i32,
    #[serde(rename = "@line")]
    pub line: i32,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OsFingerprintXml {
    #[serde(rename = "@fingerprint")]
    pub fingerprint: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SmurfXml {
    #[serde(rename = "@responses")]
    pub responses: i32,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DistanceXml {
    #[serde(rename = "@value")]
    pub value: i32,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct UptimeXml {
    #[serde(rename = "@seconds")]
    pub seconds: i64,
    #[serde(rename = "@lastboot")]
    pub lastboot: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TcpSequenceXml {
    #[serde(rename = "@index")]
    pub index: i32,
    #[serde(rename = "@difficulty")]
    pub difficulty: String,
    #[serde(rename = "@values")]
    pub values: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct IpidSequenceXml {
    #[serde(rename = "@class")]
    pub class: String,
    #[serde(rename = "@values")]
    pub values: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TcptsSequenceXml {
    #[serde(rename = "@class")]
    pub class: String,
    #[serde(rename = "@values")]
    pub values: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TraceXml {
    #[serde(rename = "@proto")]
    pub proto: Option<String>,
    #[serde(rename = "@port")]
    pub port: Option<String>,
    #[serde(rename = "hop", default)]
    pub hops: Vec<HopXml>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct HopXml {
    #[serde(rename = "@ttl")]
    pub ttl: i32,
    #[serde(rename = "@rtt")]
    pub rtt: Option<String>,
    #[serde(rename = "@ipaddr")]
    pub ipaddr: Option<String>,
    #[serde(rename = "@host")]
    pub host: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TimesXml {
    #[serde(rename = "@srtt")]
    pub srtt: String,
    #[serde(rename = "@rttvar")]
    pub rttvar: String,
    #[serde(rename = "@to")]
    pub to: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RunStatsXml {
    pub finished: Option<FinishedXml>,
    pub hosts: Option<HostsXml>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct FinishedXml {
    #[serde(rename = "@time")]
    pub time: i64,
    #[serde(rename = "@timestr")]
    pub timestr: Option<String>,
    #[serde(rename = "@elapsed")]
    pub elapsed: Option<f64>,
    #[serde(rename = "@summary")]
    pub summary: Option<String>,
    #[serde(rename = "@exit")]
    pub exit: Option<String>,
    #[serde(rename = "@errormsg")]
    pub errormsg: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct HostsXml {
    #[serde(rename = "@up")]
    pub up: i32,
    #[serde(rename = "@down")]
    pub down: i32,
    #[serde(rename = "@total")]
    pub total: i32,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OutputXml {
    #[serde(rename = "@type")]
    pub output_type: Option<String>,
    #[serde(rename = "$text")]
    pub body: Option<String>,
}

pub fn parse_nmap_xml(xml_data: &str) -> Result<NmapRunXml, quick_xml::DeError> {
    let cleaned_xml = strip_nmap_preamble(xml_data);
    quick_xml::de::from_str(&cleaned_xml)
}

fn strip_nmap_preamble(raw: &str) -> String {
    let mut out = Vec::<&str>::new();
    let mut hosthint_depth: i32 = 0;
    for line in raw.lines() {
        let t = line.trim_start();
        // Always drop DOCTYPE and xml-stylesheet PI.
        if t.starts_with("<!DOCTYPE") || t.starts_with("<?xml-stylesheet") {
            continue;
        }
        // Track entering <hosthint> blocks (opening tag is never self-closing in nmap output).
        if t.starts_with("<hosthint") && !t.starts_with("</hosthint") {
            hosthint_depth += 1;
        }
        if hosthint_depth > 0 {
            // Decrement on the closing tag line and then continue (skip it too).
            if t.starts_with("</hosthint") {
                hosthint_depth -= 1;
            }
            continue;
        }
        // Drop single-line task elements to avoid non-contiguous duplicate field errors.
        if t.starts_with("<taskbegin")
            || t.starts_with("<taskprogress")
            || t.starts_with("<taskend")
        {
            continue;
        }
        out.push(line);
    }
    out.join("\n")
}