import { invoke } from '@tauri-apps/api/core';

export async function listAdapters() {
  return invoke('list_adapters');
}

export async function enableAdapter(name) {
  return invoke('enable_adapter', { name });
}

export async function disableAdapter(name) {
  return invoke('disable_adapter', { name });
}

export async function testConnectivity() {
  return invoke('test_connectivity');
}

export async function testAdapterConnectivity(name) {
  return invoke('test_adapter_connectivity', { name });
}

export async function setRoutingMetric(interfaceIndex, metric) {
  return invoke('set_routing_metric', { interfaceIndex, metric });
}

export async function configureLoadBalancing(primaryIndex, secondaryIndex) {
  return invoke('configure_load_balancing', { primaryIndex, secondaryIndex });
}

export async function configureFailover(primaryIndex, secondaryIndex) {
  return invoke('configure_failover', { primaryIndex, secondaryIndex });
}

export async function getMonitorState() {
  return invoke('get_monitor_state');
}

export async function enableAutoFailover(primaryIndex, secondaryIndex, primaryName, secondaryName) {
  return invoke('enable_auto_failover', { primaryIndex, secondaryIndex, primaryName, secondaryName });
}

export async function disableAutoFailover() {
  return invoke('disable_auto_failover');
}

export async function getFailoverState() {
  return invoke('get_failover_state');
}

export async function getSettings() {
  return invoke('get_settings');
}

export async function saveSettings(intervalSecs, pingTarget, adapterRefreshSecs) {
  return invoke('save_settings', { intervalSecs, pingTarget, adapterRefreshSecs });
}
