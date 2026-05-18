export interface TailscalePeer {
  id: string;
  hostname: string;
  dnsName: string;
  os: string;
  tailscaleIps: string[];
  online: boolean;
  exitNodeOption: boolean;
  exitNode: boolean;
}

export interface TailscaleStatus {
  backendState: string; // "Running" | "Stopped" | "NeedsLogin" | "NoState"
  version: string;
  selfNode: TailscalePeer | null;
  peers: TailscalePeer[];
}

export type TorMode = 'off' | 'socks5' | 'transparent';

export interface TorStatus {
  active: boolean;
  mode: TorMode;
  socksReachable: boolean;
}

export interface VpnConnection {
  name: string;
  uuid: string;
  vpnType: string;
  active: boolean;
  device: string;
}
