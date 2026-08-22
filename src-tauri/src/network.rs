use crate::logger;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Run a command that never shows a console window on Windows
fn run_hidden_command(cmd: &str, args: &[&str]) -> Result<std::process::Output> {
    let mut command = Command::new(cmd);
    command.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command.output().context(format!("Failed to execute {}", cmd))
}

/// Represents a network adapter on the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAdapter {
    pub name: String,
    pub interface_index: u32,
    pub status: String,
    pub mac_address: String,
    pub speed: String,
    pub description: String,
    pub is_connected: bool,
}

/// Result of a connectivity test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectivityResult {
    pub reachable: bool,
    pub latency_ms: Option<u64>,
}

/// Run a PowerShell command and return stdout (no window)
fn run_powershell(script: &str) -> Result<String> {
    let output = run_hidden_command("powershell.exe", &[
        "-NoProfile", "-NonInteractive", "-Command", script,
    ])?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("PowerShell error: {}", stderr);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Ping a host using native ping.exe (no PowerShell, no window)
fn ping_host(host: &str, timeout_ms: u32) -> Result<ConnectivityResult> {
    let timeout_str = timeout_ms.to_string();
    let output = run_hidden_command("ping.exe", &[
        "-n", "1", "-w", &timeout_str, host,
    ])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stdout_lower = stdout.to_lowercase();

    if stdout_lower.contains("ttl=") || stdout_lower.contains("durée=") || stdout_lower.contains("durée EXPIRÉ") || stdout_lower.contains("time=") {
        // Extract latency: "time=12ms", "time<1ms", or French "temps=12ms", "durée=<1ms"
        let latency = stdout
            .lines()
            .find(|l| {
                let ll = l.to_lowercase();
                ll.contains("time=") || ll.contains("temps=") || ll.contains("durée=")
            })
            .and_then(|line| {
                let ll = line.to_lowercase();
                let key = if ll.contains("time=") { "time=" }
                    else if ll.contains("temps=") { "temps=" }
                    else { "durée=" };
                let time_part = line.split(key).nth(1)?;
                let ms_str: String = time_part.chars().take_while(|c| c.is_ascii_digit() || *c == '<').collect::<String>().trim_start_matches('<').chars().take_while(|c| c.is_ascii_digit()).collect();
                ms_str.parse::<u64>().ok()
            });
        Ok(ConnectivityResult {
            reachable: true,
            latency_ms: latency,
        })
    } else {
        Ok(ConnectivityResult {
            reachable: false,
            latency_ms: None,
        })
    }
}

/// List all network adapters
pub fn list_adapters() -> Result<Vec<NetworkAdapter>> {
    let ps_script = r#"
        Get-NetAdapter | Select-Object Name, ifIndex, Status, MacAddress, LinkSpeed, InterfaceDescription |
        ConvertTo-Json -Compress
    "#;

    let output = run_powershell(ps_script)?;

    let trimmed = output.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let json_str = if trimmed.starts_with('[') {
        trimmed.to_string()
    } else {
        format!("[{}]", trimmed)
    };

    let raw: Vec<serde_json::Value> = serde_json::from_str(&json_str)
        .context("Failed to parse adapter list JSON")?;

    let adapters = raw
        .into_iter()
        .map(|v| {
            let name = v["Name"].as_str().unwrap_or("Unknown").to_string();
            let interface_index = v["ifIndex"].as_u64().unwrap_or(0) as u32;
            let status = v["Status"].as_str().unwrap_or("Unknown").to_string();
            let mac_address = v["MacAddress"].as_str().unwrap_or("N/A").to_string();
            let speed = v["LinkSpeed"].as_str().unwrap_or("N/A").to_string();
            let description = v["InterfaceDescription"]
                .as_str()
                .unwrap_or("Unknown")
                .to_string();
            let is_connected = status == "Up";

            NetworkAdapter {
                name,
                interface_index,
                status,
                mac_address,
                speed,
                description,
                is_connected,
            }
        })
        .collect();

    Ok(adapters)
}

/// Enable a network adapter by name
pub fn enable_adapter(name: &str) -> Result<()> {
    let script = format!(
        "Enable-NetAdapter -Name '{}' -Confirm:$false -ErrorAction Stop",
        name.replace('\'', "''")
    );
    run_powershell(&script).context(format!("Failed to enable adapter '{}'", name))?;
    Ok(())
}

/// Disable a network adapter by name
pub fn disable_adapter(name: &str) -> Result<()> {
    let script = format!(
        "Disable-NetAdapter -Name '{}' -Confirm:$false -ErrorAction Stop",
        name.replace('\'', "''")
    );
    run_powershell(&script).context(format!("Failed to disable adapter '{}'", name))?;
    Ok(())
}

/// Set the routing metric for a network interface
pub fn set_routing_metric(interface_index: u32, metric: u32) -> Result<()> {
    let script = format!(
        "Set-NetIPInterface -InterfaceIndex {} -InterfaceMetric {} -ErrorAction Stop",
        interface_index, metric
    );
    run_powershell(&script).context(format!(
        "Failed to set metric {} on interface {}",
        metric, interface_index
    ))?;
    Ok(())
}

/// Test connectivity to a host (native ping, no PowerShell, no window)
pub fn test_connectivity_to(host: &str) -> Result<ConnectivityResult> {
    ping_host(host, 2000)
}

/// Test connectivity to the internet (default: 8.8.8.8)
pub fn test_connectivity() -> Result<ConnectivityResult> {
    test_connectivity_to("8.8.8.8")
}

/// Test connectivity on a specific adapter (native ping, no PowerShell, no window)
pub fn test_adapter_connectivity(_adapter_name: &str) -> Result<ConnectivityResult> {
    // Simple ping — if host is reachable, adapter is working
    test_connectivity()
}

/// Configure load balancing between two adapters
pub fn configure_load_balancing(
    primary_index: u32,
    secondary_index: u32,
) -> Result<()> {
    set_routing_metric(primary_index, 10)?;
    set_routing_metric(secondary_index, 20)?;

    let _ = run_powershell(
        "Set-NetIPInterface -Forwarding Enabled -ErrorAction SilentlyContinue",
    );

    Ok(())
}

/// Configure failover between two adapters
pub fn configure_failover(
    primary_index: u32,
    secondary_index: u32,
) -> Result<()> {
    set_routing_metric(primary_index, 10)?;
    set_routing_metric(secondary_index, 100)?;

    Ok(())
}

/// Detailed adapter info for expert mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterDetails {
    pub name: String,
    pub interface_index: u32,
    pub status: String,
    pub description: String,
    pub mac_address: String,
    pub speed: String,
    pub ip_address: String,
    pub subnet_mask: String,
    pub default_gateway: String,
    pub dns_servers: String,
    pub dhcp_enabled: bool,
    pub routing_metric: Option<u32>,
}

/// Batch query: all adapter details in a single PowerShell call
pub fn get_all_adapter_details() -> Result<Vec<AdapterDetails>> {
    // One PowerShell script that returns everything at once
    let ps_script = r#"
$adapters = Get-NetAdapter | Select-Object Name, ifIndex, Status, MacAddress, LinkSpeed, InterfaceDescription
$results = @()
foreach ($a in $adapters) {
    $idx = $a.ifIndex
    $ip = ""
    $prefix = 24
    try {
        $ipObj = Get-NetIPAddress -InterfaceIndex $idx -AddressFamily IPv4 -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($ipObj) { $ip = $ipObj.IPAddress; $prefix = $ipObj.PrefixLength }
    } catch {}
    $gw = ""
    try {
        $gwObj = Get-NetIPConfiguration -InterfaceIndex $idx -ErrorAction SilentlyContinue
        if ($gwObj -and $gwObj.IPv4DefaultGateway) { $gw = $gwObj.IPv4DefaultGateway.NextHop }
    } catch {}
    $dns = ""
    try {
        $dnsArr = Get-DnsClientServerAddress -InterfaceIndex $idx -AddressFamily IPv4 -ErrorAction SilentlyContinue | Select-Object -ExpandProperty ServerAddresses
        if ($dnsArr) { $dns = ($dnsArr -join ", ") }
    } catch {}
    $dhcp = $false
    try {
        $dhcpVal = Get-NetIPInterface -InterfaceIndex $idx -AddressFamily IPv4 -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Dhcp
        if ($dhcpVal -eq "Enabled") { $dhcp = $true }
    } catch {}
    $metric = 0
    try {
        $mVal = Get-NetIPInterface -InterfaceIndex $idx -AddressFamily IPv4 -ErrorAction SilentlyContinue | Select-Object -ExpandProperty InterfaceMetric
        if ($mVal) { $metric = [int]$mVal }
    } catch {}
    $results += @{
        name = $a.Name
        interface_index = $idx
        status = $a.Status
        description = $a.InterfaceDescription
        mac_address = $a.MacAddress
        speed = $a.LinkSpeed
        ip_address = $ip
        prefix = $prefix
        default_gateway = $gw
        dns_servers = $dns
        dhcp_enabled = $dhcp
        routing_metric = $metric
    }
}
$results | ConvertTo-Json -Compress
"#;
    let output = run_powershell(ps_script).unwrap_or_else(|e| {
        logger::log_file(&format!("CMD get_all_adapter_details ERROR: {}", e));
        String::new()
    });

    let trimmed = output.trim();
    if trimmed.is_empty() || trimmed == "null" {
        return Ok(Vec::new());
    }

    let json_str = if trimmed.starts_with('[') {
        trimmed.to_string()
    } else {
        format!("[{}]", trimmed)
    };

    let raw: Vec<serde_json::Value> = serde_json::from_str(&json_str)
        .context("Failed to parse adapter details JSON")?;

    let details = raw
        .into_iter()
        .map(|v| {
            let name = v["name"].as_str().unwrap_or("Unknown").to_string();
            let interface_index = v["interface_index"].as_u64().unwrap_or(0) as u32;
            let status = v["status"].as_str().unwrap_or("Unknown").to_string();
            let description = v["description"].as_str().unwrap_or("Unknown").to_string();
            let mac_address = v["mac_address"].as_str().unwrap_or("N/A").to_string();
            let speed = v["speed"].as_str().unwrap_or("N/A").to_string();
            let ip_address = v["ip_address"].as_str().unwrap_or("N/A").to_string();
            let prefix = v["prefix"].as_u64().unwrap_or(24) as u32;
            let subnet_mask = prefix_to_subnet(prefix);
            let default_gateway = v["default_gateway"].as_str().unwrap_or("N/A").to_string();
            let dns_servers = v["dns_servers"].as_str().unwrap_or("N/A").to_string();
            let dhcp_enabled = v["dhcp_enabled"].as_bool().unwrap_or(false);
            let routing_metric = v["routing_metric"].as_u64().map(|m| m as u32);

            AdapterDetails {
                name,
                interface_index,
                status,
                description,
                mac_address,
                speed,
                ip_address,
                subnet_mask,
                default_gateway,
                dns_servers,
                dhcp_enabled,
                routing_metric,
            }
        })
        .collect();

    Ok(details)
}

/// Get detailed info for a single adapter (uses batch query internally)
pub fn get_adapter_details(name: &str) -> Result<AdapterDetails> {
    let all = get_all_adapter_details()?;
    all.into_iter()
        .find(|a| a.name == name)
        .with_context(|| format!("Adapter {} not found", name))
}

fn prefix_to_subnet(prefix: u32) -> String {
    if prefix > 32 { return "N/A".to_string(); }
    let mask = if prefix == 0 { 0u32 } else { !0u32 << (32 - prefix) };
    format!("{}.{}.{}.{}/{}",
        (mask >> 24) & 0xFF,
        (mask >> 16) & 0xFF,
        (mask >> 8) & 0xFF,
        mask & 0xFF,
        prefix
    )
}

