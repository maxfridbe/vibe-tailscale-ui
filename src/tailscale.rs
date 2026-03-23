use serde::Deserialize;
use std::collections::HashMap;
use tokio::process::Command;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct TailscaleStatus {
    #[serde(rename = "Self")]
    pub self_node: NodeInfo,
    #[serde(default)]
    pub peer: HashMap<String, NodeInfo>,
    pub exit_node_status: Option<ExitNodeStatus>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct ExitNodeStatus {
    pub id: Option<String>,
    #[serde(default)]
    pub online: bool,
    #[serde(rename = "TailscaleIPs", default)]
    pub tailscale_ips: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct NodeInfo {
    pub id: Option<String>,
    #[serde(default)]
    pub host_name: String,
    pub dns_name: Option<String>,
    #[serde(rename = "TailscaleIPs", default)]
    pub tailscale_ips: Vec<String>,
    pub online: Option<bool>,
    #[serde(default)]
    pub exit_node: bool,
    #[serde(default)]
    pub exit_node_option: bool,
    #[serde(rename = "OS")]
    pub os: Option<String>,
    pub last_seen: Option<String>,
}

impl NodeInfo {
    pub fn sanitize(&mut self) {
        self.host_name = self.host_name.replace('\0', "");
        if let Some(dns) = self.dns_name.as_mut() {
            *dns = dns.replace('\0', "");
        }
        for ip in self.tailscale_ips.iter_mut() {
            *ip = ip.replace('\0', "");
        }
        if let Some(os) = self.os.as_mut() {
            *os = os.replace('\0', "");
        }
        if let Some(ls) = self.last_seen.as_mut() {
            *ls = ls.replace('\0', "");
        }
    }
}

impl TailscaleStatus {
    pub fn sanitize(&mut self) {
        self.self_node.sanitize();
        for node in self.peer.values_mut() {
            node.sanitize();
        }
    }
}

pub async fn get_status() -> Result<TailscaleStatus, Box<dyn std::error::Error + Send + Sync>> {
    let output = Command::new("distrobox-host-exec")
        .arg("tailscale")
        .arg("status")
        .arg("--json")
        .output()
        .await?;

    if !output.status.success() {
        return Err(format!("Tailscale command failed: {}", String::from_utf8_lossy(&output.stderr)).into());
    }

    let mut status: TailscaleStatus = serde_json::from_slice(&output.stdout)?;
    status.sanitize();
    Ok(status)
}

pub async fn set_exit_node(node_ip: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let output = Command::new("distrobox-host-exec")
        .arg("tailscale")
        .arg("up")
        .arg("--exit-node")
        .arg(node_ip)
        .output()
        .await?;

    if !output.status.success() {
        return Err(format!("Tailscale up failed: {}", String::from_utf8_lossy(&output.stderr)).into());
    }

    Ok(())
}

pub async fn disable_exit_node() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let output = Command::new("distrobox-host-exec")
        .arg("tailscale")
        .arg("up")
        .arg("--exit-node=")
        .output()
        .await?;

    if !output.status.success() {
        return Err(format!("Tailscale up failed: {}", String::from_utf8_lossy(&output.stderr)).into());
    }

    Ok(())
}
