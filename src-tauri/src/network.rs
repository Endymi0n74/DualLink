use crate::logger;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Timeout des tests de connectivité (argument `-w` de ping.exe).
const PING_TIMEOUT_MS: u32 = 2000;

/// Cible de test par défaut, également utilisée par les commandes ponctuelles.
pub const DEFAULT_PING_TARGET: &str = "8.8.8.8";

/// Métrique appliquée au primaire en fonctionnement nominal.
pub const METRIC_PRIMARY: u32 = 10;
/// Métrique appliquée au secondaire en fonctionnement nominal.
pub const METRIC_SECONDARY: u32 = 100;

/// Run a command that never shows a console window on Windows
fn run_hidden_command(cmd: &str, args: &[&str]) -> Result<std::process::Output> {
    let mut command = Command::new(cmd);
    command.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
        .output()
        .context(format!("Failed to execute {}", cmd))
}

/// Échappe une valeur destinée à une chaîne PowerShell entre apostrophes.
fn escape_ps(value: &str) -> String {
    value.replace('\'', "''")
}

/// Représente un adaptateur réseau du système
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
    let output = run_hidden_command(
        "powershell.exe",
        &["-NoProfile", "-NonInteractive", "-Command", script],
    )?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("PowerShell error: {}", stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

// ── Tests de connectivité ─────────────────────────────────────────

/// Extrait la latence d'une sortie `ping.exe`, en français ou en anglais.
///
/// Formates gérés: `time=12ms`, `time<1ms`, `temps=12 ms`, `temps<1 ms`,
/// `durée=14 ms`, `durée<1 ms`.
///
/// Les caractères non-ASCII sont retirés avant analyse: la sortie de `ping.exe`
/// suit la page de code de la console (cp437/cp850 sur un Windows français), et
/// le décodage UTF-8 transforme alors les accents en caractères de remplacement.
pub fn parse_latency(ping_output: &str) -> Option<u64> {
    for line in ping_output.lines() {
        let ascii: String = line.chars().filter(char::is_ascii).collect();
        let lower = ascii.to_lowercase();

        for keyword in ["temps", "time", "dure"] {
            let Some(pos) = lower.find(keyword) else {
                continue;
            };
            let tail = &lower[pos + keyword.len()..];

            // Cherche le "=" ou le "<" qui suit le mot-clé, puis les chiffres.
            let Some(sep) = tail.find(['=', '<']) else {
                continue;
            };
            let digits: String = tail[sep + 1..]
                .trim_start()
                .trim_start_matches(['<', '='])
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();

            if let Ok(ms) = digits.parse::<u64>() {
                return Some(ms);
            }
        }
    }
    None
}

/// Interprète une sortie `ping.exe` et son code de sortie.
///
/// Un ping n'est réussi que si le code de sortie vaut 0 **et** qu'une latence a
/// été parsée: sans latence, impossible de distinguer une vraie réponse d'une
/// sortie tronquée ou inattendue.
pub fn parse_ping_result(ping_output: &str, exit_success: bool) -> ConnectivityResult {
    let latency_ms = parse_latency(ping_output);
    ConnectivityResult {
        reachable: exit_success && latency_ms.is_some(),
        latency_ms,
    }
}

/// Vérifie qu'une cible de ping est sûre à passer en argument de processus.
///
/// N'accepte que des adresses IPv4 et des noms d'hôtes DNS: cela exclut les
/// métacaractères du shell (`&`, `|`, `;`, `"`…) qui seraient interprétés par
/// `ping.exe` ou par PowerShell.
pub fn is_valid_ping_target(target: &str) -> bool {
    !target.is_empty()
        && target.len() <= 255
        && target
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':'))
}

/// Ping a host using native ping.exe (no PowerShell, no window)
///
/// `source_ip` force l'adresse source, mais ne suffit pas à sélectionner
/// l'interface: Windows peut router le paquet par la route par défaut.
fn ping_host(host: &str, timeout_ms: u32, source_ip: Option<&str>) -> Result<ConnectivityResult> {
    if !is_valid_ping_target(host) {
        return Err(anyhow!("Invalid ping target '{host}'"));
    }

    let timeout_str = timeout_ms.to_string();
    let mut args: Vec<&str> = vec!["-n", "1", "-w", &timeout_str];
    if let Some(src) = source_ip.filter(|s| !s.is_empty() && *s != "N/A") {
        args.push("-S");
        args.push(src);
    }
    args.push(host);

    let output = run_hidden_command("ping.exe", &args)?;
    Ok(parse_ping_result(
        &String::from_utf8_lossy(&output.stdout),
        output.status.success(),
    ))
}

/// Test connectivity to a host (native ping, no PowerShell, no window)
pub fn test_connectivity_to(host: &str) -> Result<ConnectivityResult> {
    ping_host(host, PING_TIMEOUT_MS, None)
}

/// Test connectivity to the internet (default: 8.8.8.8)
pub fn test_connectivity() -> Result<ConnectivityResult> {
    test_connectivity_to(DEFAULT_PING_TARGET)
}

/// Test connectivity on a specific adapter
///
/// L'interface est réellement testée: la route vers la cible est épinglée sur
/// cette interface avant le ping (cf. `ping_via_interface`).
pub fn test_adapter_connectivity(adapter_name: &str) -> Result<ConnectivityResult> {
    let route = RouteTarget::from_adapter_name(adapter_name)?;
    ping_via_interface(DEFAULT_PING_TARGET, &route)
}

// ── Routage forcé (route hôte épinglée) ───────────────────────────

//     Cible ──> table de routage Windows ──┬── route par défaut  (metrique 10/100)
//                                          │      → primaire ou secondaire
//                                          └── route hôte /32   (RouteMetric 1)
//                                                 → interface de ce RouteTarget
//
/// Interface à emprunter pour atteindre une cible, avec sa passerelle.
///
/// La route hôte est plus spécifique que la route par défaut: le trafic est
/// donc envoyé par cette interface même lorsque la métrique d'interface du
/// secondaire est plus basse. C'est ce qui permet de tester le primaire
/// réellement pendant un failover, au lieu de tester le secondaire par erreur.
#[derive(Debug, Clone)]
pub struct RouteTarget {
    pub interface_index: u32,
    pub source_ip: Option<String>,
    pub gateway: Option<String>,
}

impl RouteTarget {
    /// Résout une interface par son nom, en vue de ses identifiants de route.
    pub fn from_adapter_name(name: &str) -> Result<RouteTarget> {
        let details = get_adapter_details(name)?;
        Ok(RouteTarget {
            interface_index: details.interface_index,
            source_ip: Some(details.ip_address),
            gateway: Some(details.default_gateway),
        })
    }
}

/// Crée une route hôte `target/32` sur l'interface donnée, métrique 1.
///
/// Si la route existe déjà, elle est remplacée: sinon Windows refuse le
/// `New-NetRoute` et le test qui suit measure à nouveau la route par défaut.
pub fn pin_route_via(target: &str, interface_index: u32, gateway: &str) -> Result<()> {
    let prefix = format!("{target}/32");
    let script = format!(
        "Remove-NetRoute -DestinationPrefix '{prefix}' -InterfaceIndex {interface_index} \
         -Confirm:$false -ErrorAction SilentlyContinue; \
         New-NetRoute -DestinationPrefix '{prefix}' -InterfaceIndex {interface_index} \
         -NextHop '{gateway}' -RouteMetric 1 -PolicyStore ActiveStore -ErrorAction Stop",
        prefix = escape_ps(&prefix),
        gateway = escape_ps(gateway)
    );
    run_powershell(&script).with_context(|| {
        format!("Failed to pin route to {target} via interface {interface_index}")
    })?;
    Ok(())
}

/// Supprime la route hôte créée par `pin_route_via`.
///
/// Sans effet si la route n'existe pas: le retrait est toujours tenté, y
/// compris quand le ping a échoué ou que l'interface est tombée.
pub fn unpin_route(target: &str, interface_index: u32) -> Result<()> {
    let prefix = format!("{target}/32");
    let script = format!(
        "Remove-NetRoute -DestinationPrefix '{}' -InterfaceIndex {} \
         -Confirm:$false -ErrorAction SilentlyContinue",
        escape_ps(&prefix),
        interface_index
    );
    run_powershell(&script).with_context(|| {
        format!("Failed to unpin route to {target} via interface {interface_index}")
    })?;
    Ok(())
}

/// Ping `target` en forçant le passage par `route`.
///
/// La route est épinglée avant le ping et retirée après, quelle que soit
/// l'issue. Si l'épinglage échoue (interface absente, passerelle inconnue),
/// aucune mesure n'est renvoyée: continuer à mesurer la route par défaut
/// donnerait un résultat trompeur.
pub fn ping_via_interface(target: &str, route: &RouteTarget) -> Result<ConnectivityResult> {
    if !is_valid_ping_target(target) {
        return Err(anyhow!("Invalid ping target '{target}'"));
    }

    let gateway = route.gateway.as_deref().unwrap_or("").trim();
    let source_ip = route
        .source_ip
        .as_deref()
        .filter(|s| !s.is_empty() && *s != "N/A");

    if gateway.is_empty() || gateway == "N/A" {
        return Err(anyhow!(
            "No default gateway on interface {}",
            route.interface_index
        ));
    }

    pin_route_via(target, route.interface_index, gateway)?;
    let result = ping_host(target, PING_TIMEOUT_MS, source_ip);

    // Le retrait est toujours tenté: une route laissée en place fausserait
    // toutes les mesures suivantes, y compris le ping global du monitor.
    if let Err(e) = unpin_route(target, route.interface_index) {
        logger::log_file(&format!(
            "WARN unpin route '{}' interface {}: {:#}",
            target, route.interface_index, e
        ));
    }

    result
}

/// Teste plusieurs interfaces, chacune par sa propre route épinglée.
///
/// Les sondes sont séquentielles: plusieurs routes `/32` vers la même cible
/// coexistent, mais le choix de celle qui sert au ping suit la plus faible
/// métrique, donc une seule interface est réellement testée à la fois.
pub fn probe_routes(target: &str, routes: &[RouteTarget]) -> Vec<(u32, ConnectivityResult)> {
    routes
        .iter()
        .map(|r| {
            let res = ping_via_interface(target, r).unwrap_or_else(|e| {
                logger::log_file(&format!(
                    "WARN probe interface {} error: {:#}",
                    r.interface_index, e
                ));
                ConnectivityResult {
                    reachable: false,
                    latency_ms: None,
                }
            });
            (r.interface_index, res)
        })
        .collect()
}

// ── Gestion des adaptateurs ──────────────────────────────────────

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

    let raw: Vec<serde_json::Value> =
        serde_json::from_str(&json_str).context("Failed to parse adapter list JSON")?;

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
    set_adapter_enabled(name, true)
}

/// Disable a network adapter by name
pub fn disable_adapter(name: &str) -> Result<()> {
    set_adapter_enabled(name, false)
}

/// Active ou désactive un adaptateur.
///
/// Le nom est d'abord résolu en index d'interface: les noms d'adaptateurs sont
/// traduits selon la locale (`Ethernet`, `Ethernet 2`, `Connexion réseau
/// local`…), l'index ne l'est pas. Le nom reste utilisé pour la résolution,
/// d'où l'échappement PowerShell.
fn set_adapter_enabled(name: &str, enabled: bool) -> Result<()> {
    let verb = if enabled { "Enable" } else { "Disable" };
    let action = if enabled { "enable" } else { "disable" };
    let script = format!(
        "$if = Get-NetAdapter -Name '{}' -ErrorAction Stop | Select-Object -First 1; \
         if (-not $if) {{ throw 'Adapter not found' }}; \
         {}-NetAdapter -InterfaceIndex $if.ifIndex -Confirm:$false -ErrorAction Stop",
        escape_ps(name),
        verb
    );
    run_powershell(&script).with_context(|| format!("Failed to {action} adapter '{name}'"))?;
    Ok(())
}

/// Set the routing metric for a network interface
///
/// `-AutomaticMetric Disabled` est indispensable: sans lui, Windows continue
/// d'appliquer la métrique automatique et ignore `-InterfaceMetric`.
pub fn set_routing_metric(interface_index: u32, metric: u32) -> Result<()> {
    let script = format!(
        "Set-NetIPInterface -InterfaceIndex {interface_index} -AddressFamily IPv4 \
         -AutomaticMetric Disabled -InterfaceMetric {metric} -ErrorAction Stop"
    );
    run_powershell(&script).context(format!(
        "Failed to set metric {metric} on interface {interface_index}"
    ))?;
    Ok(())
}

/// Applique une répartition de priorité entre deux adaptateurs.
///
/// Ce n'est **pas** du load balancing: une destination donnée ne suit qu'une
/// seule interface, celle de plus faible métrique. On obtient donc une
/// répartition de priorité (le primaire garde la route par défaut, le
/// secondaire ne sert qu'en secours), pas une répartition de charge.
///
/// Le forwarding global n'est volontairement pas activé: il transformerait la
/// machine en routeur sur toutes ses interfaces, ce qui n'est pas l'effet
/// recherché ici.
pub fn configure_load_balancing(primary_index: u32, secondary_index: u32) -> Result<()> {
    set_routing_metric(primary_index, METRIC_PRIMARY)?;
    set_routing_metric(secondary_index, METRIC_SECONDARY)?;
    Ok(())
}

/// Configure failover between two adapters
pub fn configure_failover(primary_index: u32, secondary_index: u32) -> Result<()> {
    set_routing_metric(primary_index, METRIC_PRIMARY)?;
    set_routing_metric(secondary_index, METRIC_SECONDARY)?;
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
    let output = run_powershell(ps_script).context("Failed to query adapter details")?;

    let trimmed = output.trim();
    if trimmed.is_empty() || trimmed == "null" {
        return Ok(Vec::new());
    }

    let json_str = if trimmed.starts_with('[') {
        trimmed.to_string()
    } else {
        format!("[{}]", trimmed)
    };

    let raw: Vec<serde_json::Value> =
        serde_json::from_str(&json_str).context("Failed to parse adapter details JSON")?;

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
            let subnet_mask = prefix_to_mask(prefix);
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
        .with_context(|| format!("Adapter {name} not found"))
}

/// Convertit un préfixe CIDR en masque de sous-réseau pointé.
/// Le champ exposé au frontend est `subnet_mask`: on renvoie donc
/// `255.255.255.0` pour un préfixe 24, et non `255.255.255.0/24`.
fn prefix_to_mask(prefix: u32) -> String {
    if prefix > 32 {
        return "N/A".to_string();
    }
    let mask = if prefix == 0 {
        0u32
    } else {
        u32::MAX << (32 - prefix)
    };
    format!(
        "{}.{}.{}.{}",
        (mask >> 24) & 0xFF,
        (mask >> 16) & 0xFF,
        (mask >> 8) & 0xFF,
        mask & 0xFF
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const PING_EN: &str = "Pinging 8.8.8.8 with 32 bytes of data:\n\
                            Reply from 8.8.8.8: bytes=32 time=14ms TTL=117\n";

    const PING_EN_SUB_MS: &str = "Reply from 192.168.1.254: bytes=32 time<1ms TTL=64\n";

    const PING_FR: &str = "ping vers 8.8.8.8 avec 32 octets de données :\n\
                            Réponse de 8.8.8.8 : octets=32 temps=14 ms TTL=117\n";

    const PING_FR_SUB_MS: &str = "Réponse de 192.168.1.254 : octets=32 temps<1 ms TTL=64\n";

    const PING_FR_DUREE: &str = "Réponse de 8.8.8.8 : octets=32 durée=23 ms TTL=116\n";

    const PING_FR_DUREE_SUB_MS: &str = "Réponse de 8.8.8.8 : octets=32 durée<1 ms TTL=116\n";

    const PING_FAIL_FR: &str = "ping vers 8.8.8.8 avec 32 octets de données :\n\
                                Délai d'attente de la demande dépassé.\n";

    const PING_FAIL_EN: &str = "Pinging 8.8.8.8 with 32 bytes of data:\nRequest timed out.\n";

    /// Sortie réelle d'un Windows FR (page de code console cp850), après
    /// `String::from_utf8_lossy`: les accents deviennent U+FFFD.
    fn lossy(bytes: &[u8]) -> String {
        String::from_utf8_lossy(bytes).to_string()
    }

    #[test]
    fn parse_sortie_reelle_windows_francais() {
        // Capturé sur un Windows 11 FR, encodage console cp850.
        let ok = b"Envoi d'une requ\x82te 'Ping'  8.8.8.8 avec 32 octets de donn\x82es :\r\n\
                   R\x82ponse de 8.8.8.8 : octets=32 temps=17 ms TTL=118\r\n";
        assert_eq!(parse_latency(&lossy(ok)), Some(17));
        assert!(parse_ping_result(&lossy(ok), true).reachable);

        // Échec: aucun mot-clé de latence, code de sortie 1.
        let ko = b"D\x82lai d'attente de la demande d\x82pass\x82e.\r\n";
        assert_eq!(parse_latency(&lossy(ko)), None);
        let res = parse_ping_result(&lossy(ko), false);
        assert!(!res.reachable);
        assert_eq!(res.latency_ms, None);
    }

    #[test]
    fn parse_latence_ping_anglais() {
        assert_eq!(parse_latency(PING_EN), Some(14));
        assert_eq!(parse_latency(PING_EN_SUB_MS), Some(1));
    }

    #[test]
    fn parse_latence_ping_francais() {
        assert_eq!(parse_latency(PING_FR), Some(14));
        assert_eq!(parse_latency(PING_FR_SUB_MS), Some(1));
        assert_eq!(parse_latency(PING_FR_DUREE), Some(23));
        assert_eq!(parse_latency(PING_FR_DUREE_SUB_MS), Some(1));
    }

    #[test]
    fn parse_latence_sortie_cp850_non_utf8() {
        // Les octets 0x82 correspondent à 'é' en cp850/cp1252.
        let raw = b"R\x82ponse de 8.8.8.8 : octets=32 dur\x82e=23 ms TTL=116";
        assert_eq!(parse_latency(&lossy(raw)), Some(23));

        let raw = b"Reponse de 8.8.8.8 : octets=32 dur\x82e<1 ms TTL=116";
        assert_eq!(parse_latency(&lossy(raw)), Some(1));
    }

    #[test]
    fn parse_latence_aucune_dans_echec() {
        assert_eq!(parse_latency(PING_FAIL_FR), None);
        assert_eq!(parse_latency(PING_FAIL_EN), None);
        assert_eq!(parse_latency(""), None);
    }

    #[test]
    fn parse_resultat_exige_code_sortie_et_latence() {
        let ok_en = parse_ping_result(PING_EN, true);
        assert!(ok_en.reachable);
        assert_eq!(ok_en.latency_ms, Some(14));

        let ok_fr = parse_ping_result(PING_FR, true);
        assert!(ok_fr.reachable);
        assert_eq!(ok_fr.latency_ms, Some(14));

        // Code 0 mais aucune latence: sortie tronquée, résultat non fiable.
        let truncated = parse_ping_result("Pinging 8.8.8.8 ...\n", true);
        assert!(!truncated.reachable);
        assert_eq!(truncated.latency_ms, None);

        // Latence lisible mais code de sortie non nul: échec.
        assert!(!parse_ping_result(PING_FAIL_FR, false).reachable);
        assert!(!parse_ping_result(PING_EN, false).reachable);
    }

    #[test]
    fn prefix_vers_masque() {
        assert_eq!(prefix_to_mask(24), "255.255.255.0");
        assert_eq!(prefix_to_mask(25), "255.255.255.128");
        assert_eq!(prefix_to_mask(16), "255.255.0.0");
        assert_eq!(prefix_to_mask(8), "255.0.0.0");
        assert_eq!(prefix_to_mask(30), "255.255.255.252");
        assert_eq!(prefix_to_mask(32), "255.255.255.255");
        assert_eq!(prefix_to_mask(0), "0.0.0.0");
        assert_eq!(prefix_to_mask(33), "N/A");
    }

    #[test]
    fn cible_ping_valide_refuse_metacaracteres() {
        assert!(is_valid_ping_target("8.8.8.8"));
        assert!(is_valid_ping_target("dns.google"));
        assert!(!is_valid_ping_target(""));
        assert!(!is_valid_ping_target("8.8.8.8 & calc"));
        assert!(!is_valid_ping_target("a|b"));
        assert!(!is_valid_ping_target("a\"b"));
        assert!(!is_valid_ping_target("a;b"));
        assert!(!is_valid_ping_target("$(whoami)"));
    }
}
