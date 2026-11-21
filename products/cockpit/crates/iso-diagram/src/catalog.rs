//! Built-in strongly typed reference scenes written with the `iso_diagram!` Rust eDSL.

use crate::iso_diagram;
use crate::model::Diagram;

/// Looks up one of the built-in showcase scenes by CLI/UI scene key.
pub fn by_name(name: &str) -> Option<Diagram> {
    match name {
        "campus-ha" | "campus_ha" => Some(campus_ha()),
        "payment-gateway" | "payment_gateway" => Some(payment_gateway()),
        "asset-gallery" | "asset_gallery" => Some(asset_gallery()),
        _ => None,
    }
}

/// Multi-site edge and core high-availability cluster reference topology.
pub fn campus_ha() -> Diagram {
    iso_diagram! {
        title: "Edge & Core HA Campus",
        theme: WarmPaper,

        node wan_west = Cloud => ("WAN West", "Fiber Uplink A") [Cobalt];
        node wan_core = Cloud => ("WAN Core", "Fiber Uplink B") [Cobalt];

        zone edge_zone ("Edge Zone", Solid, Pine) {
            node edge_gw = Router => ("Edge Gateway", "Overlay Tunnel");
            node edge_vault = rack(3) => ("Edge Backup", "Incremental Store");
            node edge_pod = Pod => ("Edge Pod", "Mesh Node");
        }

        zone work_zone ("Workstation Bay", Solid, Slate) {
            node work_pod = Pod => ("Relay Pod", "Mesh Backhaul");
            node ctl_pod = Pod => ("Telemetry Agent", "Local Node");
            node dev_laptop = Laptop => ("Workstation", "Operator Client");
        }

        node fw_core = Firewall => ("Policy Firewall", "Edge Ruleset") [Crimson];
        node sw_trunk = switch(8) => ("Core Switch", "8-Port 2.5G") [Cobalt];
        node sw_vlan = switch(8) => ("Access Switch", "Tagged VLANs") [Cobalt];

        zone compute_cluster ("Compute Cluster HA", Solid, Amber) {
            node node_01 = rack_pedestal(3) => ("node-01", "Compute Host") [Amber];
            node node_02 = rack_pedestal(3) => ("node-02", "Compute Host") [Amber];
            node node_03 = rack_pedestal(3) => ("node-03", "Compute Host") [Amber];

            zone storage_pool ("Distributed Storage", Solid, Amber) {
                node nvme_wal = drum(3) => ("NVMe Journal", "WAL Tier");
                node osd_a = drum(3) => ("Pool A", "Replication Tier");
                node osd_b = drum(3) => ("Pool B", "Archive Tier");
            }

            zone vault_zone ("Vault Backups", Dashed, Cobalt) {
                node vault_srv = rack(2) => ("Backup Vault", "Mirror Pool");
                node smb_bin = Crate => ("Snapshot Bin", "Object Share");
            }
        }

        edges {
            wan_west ~> wan_core [Amber] : "Overlay Tunnel";
            wan_west -> edge_gw [Cobalt] : "WAN";
            edge_gw -> (edge_vault, edge_pod) [Pine];
            edge_pod ..> work_pod [Cobalt] : "mesh";
            work_pod -> (ctl_pod, dev_laptop) [Slate];

            wan_core -> fw_core [Cobalt] : "1G Uplink";
            fw_core -> sw_trunk [Cobalt] : "LAN trunk";
            sw_trunk -> sw_vlan [Cobalt];
            sw_trunk -> (node_01, node_02, node_03) [Amber] : "2.5G Cluster";
            node_01 -> nvme_wal [Amber];
            node_02 -> osd_a [Amber];
            node_03 -> osd_b [Amber];
            node_03 -> vault_srv [Cobalt] : "sync";
            vault_srv -> smb_bin [Cobalt];
        }
    }
}

/// Payment processing topology with encrypted hops and redundant origins.
pub fn payment_gateway() -> Diagram {
    iso_diagram! {
        title: "Encrypted Payment Gateway",
        theme: WarmPaper,

        node user_laptop = Laptop => ("End User", "Web Checkout") [Pine];
        node pos_client = Pod => ("Edge Terminal", "POS Client") [Amber];
        node card_chip = Chip => ("Payment Card", "EMV Token") [Cobalt];
        node tls_lock = Lock => ("TLS / SSL", "mTLS Termination") [Amber];
        node lb = Balancer => ("Load Balancer", "Active-Active") [Plum];
        node origin_a = rack_pedestal(3) => ("Server A", "Ledger Core") [Cobalt];
        node origin_b = rack(3) => ("Server B", "Ledger Replica") [Cobalt];

        edges {
            user_laptop -> card_chip [Pine] : "card input";
            pos_client -> card_chip [Amber] : "tap";
            card_chip -> tls_lock [Cobalt] : "token";
            tls_lock -> lb [Cobalt] : "TLS 1.3";
            lb -> origin_a [Crimson] : "gRPC";
            lb -> origin_b [Plum] : "gRPC";
        }
    }
}

/// Complete gallery presenting all 12 procedural 2.5D hardware assets side-by-side.
pub fn asset_gallery() -> Diagram {
    iso_diagram! {
        title: "2.5D Hardware Asset Library (12 Sprites)",
        theme: WarmPaper,

        zone compute_storage ("Compute & Storage", Solid, Amber) {
            node a_rack = rack_pedestal(3) => ("Rack", "blades: 3, pedestal");
            node a_drum = drum(3) => ("Drum", "rings: 3 store");
            node a_crate = Crate => ("Crate", "Cartridge Bin");
            node a_pod = Pod => ("Pod", "Mesh / SBC");
            node a_chip = Chip => ("Chip", "Enclave / Card");
            node a_bal = Balancer => ("Balancer", "Traffic Splitter");
        }

        zone net_security ("Network, Security & Edge", Solid, Cobalt) {
            node a_fw = Firewall => ("Firewall", "Brick Policy") [Crimson];
            node a_sw = switch(8) => ("Switch", "8-Port 2.5G");
            node a_router = Router => ("Router", "Dual Antenna");
            node a_laptop = Laptop => ("Laptop", "Workstation");
            node a_lock = Lock => ("Lock", "TLS / Key") [Amber];
            node a_cloud = Cloud => ("Cloud", "WAN Gateway");
        }

        edges {
            a_cloud -> a_fw [Cobalt] : "ingress";
            a_fw -> a_sw [Cobalt] : "trunk";
            a_sw -> a_bal [Cobalt];
            a_bal -> a_rack [Amber] : "compute";
            a_rack -> a_drum [Amber] : "storage";
        }
    }
}
