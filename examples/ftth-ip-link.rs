use std::io::{self, ErrorKind};
use std::net::{Ipv4Addr, Ipv6Addr};

use clap::{Args, Parser, Subcommand};
use ftth_rtnl::{
    Gre6Config, GreConfig, Ip6TnlConfig, IpIpConfig, RtnlClient, VirtualInterfaceDelete,
    VirtualInterfaceKind, VirtualInterfaceSpec, VirtualInterfaceUpdate, VlanConfig, link::MacAddr,
};

#[derive(Parser)]
#[command(author, version, about = "Minimal link management utility built on ftth-rtnl", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Clone)]
enum VirtualInterfaceCommand {
    Create(VirtualInterfaceCreateArgs),
    Configure(VirtualInterfaceConfigureArgs),
    Delete(VirtualInterfaceDeleteArgs),
}

#[derive(Args, Clone)]
struct VirtualInterfaceCreateArgs {
    #[command(subcommand)]
    kind: VirtualInterfaceCreateKind,
}

#[derive(Subcommand, Clone)]
enum VirtualInterfaceCreateKind {
    Gre(GreArgs),
    Gretap(GreArgs),
    Ip6Gre(Gre6Args),
    Ip6Gretap(Gre6Args),
    IpIp(IpIpArgs),
    Ip6Tnl(Ip6TnlArgs),
    Vlan(VlanArgs),
}

#[derive(Args, Clone)]
struct VirtualInterfaceConfigureArgs {
    #[command(flatten)]
    target: VirtualInterfaceTarget,
    #[command(subcommand)]
    kind: VirtualInterfaceCreateKind,
    #[arg(long)]
    new_name: Option<String>,
    #[arg(long, value_parser = parse_bool_flag)]
    admin_up: Option<bool>,
}

#[derive(Args, Clone)]
struct VirtualInterfaceDeleteArgs {
    #[command(flatten)]
    target: VirtualInterfaceTarget,
}

#[derive(Args, Clone)]
struct VirtualInterfaceTarget {
    #[arg(long, conflicts_with = "name")]
    if_id: Option<u32>,
    #[arg(long, conflicts_with = "if_id")]
    name: Option<String>,
}

#[derive(Args, Clone)]
struct GreArgs {
    #[arg(long)]
    name: Option<String>,
    #[arg(long, value_name = "DEV")]
    interface: Option<String>,
    #[arg(long)]
    local: Ipv4Addr,
    #[arg(long)]
    remote: Ipv4Addr,
    #[arg(long)]
    ttl: Option<u8>,
    #[arg(long)]
    tos: Option<u8>,
    #[arg(long)]
    key: Option<u32>,
    #[arg(long)]
    encap_limit: Option<u8>,
    #[arg(long, value_parser = parse_bool_flag, default_value = "true")]
    pmtudisc: bool,
    #[arg(long, value_parser = parse_bool_flag, default_value = "false")]
    ignore_df: bool,
    #[arg(long, value_parser = parse_bool_flag)]
    up: Option<bool>,
}

#[derive(Args, Clone)]
struct Gre6Args {
    #[arg(long)]
    name: Option<String>,
    #[arg(long, value_name = "DEV")]
    interface: Option<String>,
    #[arg(long)]
    local: Ipv6Addr,
    #[arg(long)]
    remote: Ipv6Addr,
    #[arg(long)]
    hop_limit: Option<u8>,
    #[arg(long)]
    traffic_class: Option<u8>,
    #[arg(long)]
    key: Option<u32>,
    #[arg(long)]
    encap_limit: Option<u8>,
    #[arg(long, value_parser = parse_bool_flag, default_value = "true")]
    pmtudisc: bool,
    #[arg(long, value_parser = parse_bool_flag, default_value = "false")]
    ignore_df: bool,
    #[arg(long, value_parser = parse_bool_flag)]
    up: Option<bool>,
}

#[derive(Args, Clone)]
struct IpIpArgs {
    #[arg(long)]
    name: Option<String>,
    #[arg(long, value_name = "DEV")]
    interface: Option<String>,
    #[arg(long)]
    local: Ipv4Addr,
    #[arg(long)]
    remote: Ipv4Addr,
    #[arg(long)]
    ttl: Option<u8>,
    #[arg(long)]
    tos: Option<u8>,
    #[arg(long)]
    encap_limit: Option<u8>,
    #[arg(long, value_parser = parse_bool_flag, default_value = "true")]
    pmtudisc: bool,
    #[arg(long, value_parser = parse_bool_flag)]
    up: Option<bool>,
}

#[derive(Args, Clone)]
struct Ip6TnlArgs {
    #[arg(long)]
    name: Option<String>,
    #[arg(long, value_name = "DEV")]
    interface: Option<String>,
    #[arg(long)]
    local: Ipv6Addr,
    #[arg(long)]
    remote: Ipv6Addr,
    #[arg(long)]
    hop_limit: Option<u8>,
    #[arg(long)]
    traffic_class: Option<u8>,
    #[arg(long)]
    flow_label: Option<u32>,
    #[arg(long)]
    encap_limit: Option<u8>,
    #[arg(long, value_parser = parse_bool_flag, default_value = "true")]
    pmtudisc: bool,
    #[arg(long, value_parser = parse_bool_flag)]
    up: Option<bool>,
}

#[derive(Args, Clone)]
struct VlanArgs {
    #[arg(long)]
    name: Option<String>,
    #[arg(long, value_name = "DEV")]
    interface: Option<String>,
    #[arg(long)]
    vlan_id: Option<u16>,
    #[arg(long, value_parser = parse_bool_flag)]
    up: Option<bool>,
}

impl VirtualInterfaceTarget {
    fn to_delete(&self) -> io::Result<VirtualInterfaceDelete> {
        if let Some(if_id) = self.if_id {
            Ok(VirtualInterfaceDelete::ByIndex(if_id))
        } else if let Some(name) = &self.name {
            Ok(VirtualInterfaceDelete::ByName(name.clone()))
        } else {
            Err(io::Error::other("Specify --if-id or --name"))
        }
    }

    fn resolve_index(&self, client: &RtnlClient) -> io::Result<u32> {
        if let Some(if_id) = self.if_id {
            Ok(if_id)
        } else if let Some(name) = &self.name {
            Ok(client.link().interface_get_by_name(name)?.if_id)
        } else {
            Err(io::Error::other("Specify --if-id or --name"))
        }
    }
}

struct VirtualInterfaceBuild {
    name: Option<String>,
    admin_up: Option<bool>,
    kind: VirtualInterfaceKind,
}

fn build_virtual_interface_kind(
    client: &RtnlClient,
    kind: VirtualInterfaceCreateKind,
) -> io::Result<VirtualInterfaceBuild> {
    match kind {
        VirtualInterfaceCreateKind::Gre(args) => {
            let link = resolve_optional_link(client, args.interface.as_deref())?;
            let config = GreConfig {
                local: args.local,
                remote: args.remote,
                ttl: args.ttl,
                tos: args.tos,
                key: args.key,
                encap_limit: args.encap_limit,
                pmtudisc: args.pmtudisc,
                ignore_df: args.ignore_df,
                link,
            };
            Ok(VirtualInterfaceBuild {
                name: args.name.clone(),
                admin_up: args.up,
                kind: VirtualInterfaceKind::Gre(config),
            })
        }
        VirtualInterfaceCreateKind::Gretap(args) => {
            let link = resolve_optional_link(client, args.interface.as_deref())?;
            let config = GreConfig {
                local: args.local,
                remote: args.remote,
                ttl: args.ttl,
                tos: args.tos,
                key: args.key,
                encap_limit: args.encap_limit,
                pmtudisc: args.pmtudisc,
                ignore_df: args.ignore_df,
                link,
            };
            Ok(VirtualInterfaceBuild {
                name: args.name.clone(),
                admin_up: args.up,
                kind: VirtualInterfaceKind::Gretap(config),
            })
        }
        VirtualInterfaceCreateKind::Ip6Gre(args) => {
            let link = resolve_optional_link(client, args.interface.as_deref())?;
            let config = Gre6Config {
                local: args.local,
                remote: args.remote,
                hop_limit: args.hop_limit,
                traffic_class: args.traffic_class,
                key: args.key,
                encap_limit: args.encap_limit,
                pmtudisc: args.pmtudisc,
                ignore_df: args.ignore_df,
                link,
            };
            Ok(VirtualInterfaceBuild {
                name: args.name.clone(),
                admin_up: args.up,
                kind: VirtualInterfaceKind::Ip6Gre(config),
            })
        }
        VirtualInterfaceCreateKind::Ip6Gretap(args) => {
            let link = resolve_optional_link(client, args.interface.as_deref())?;
            let config = Gre6Config {
                local: args.local,
                remote: args.remote,
                hop_limit: args.hop_limit,
                traffic_class: args.traffic_class,
                key: args.key,
                encap_limit: args.encap_limit,
                pmtudisc: args.pmtudisc,
                ignore_df: args.ignore_df,
                link,
            };
            Ok(VirtualInterfaceBuild {
                name: args.name.clone(),
                admin_up: args.up,
                kind: VirtualInterfaceKind::Ip6Gretap(config),
            })
        }
        VirtualInterfaceCreateKind::IpIp(args) => {
            let link = resolve_optional_link(client, args.interface.as_deref())?;
            let config = IpIpConfig {
                local: args.local,
                remote: args.remote,
                ttl: args.ttl,
                tos: args.tos,
                encap_limit: args.encap_limit,
                pmtudisc: args.pmtudisc,
                link,
            };
            Ok(VirtualInterfaceBuild {
                name: args.name.clone(),
                admin_up: args.up,
                kind: VirtualInterfaceKind::IpIp(config),
            })
        }
        VirtualInterfaceCreateKind::Ip6Tnl(args) => {
            let link = resolve_optional_link(client, args.interface.as_deref())?;
            let config = Ip6TnlConfig {
                local: args.local,
                remote: args.remote,
                hop_limit: args.hop_limit,
                traffic_class: args.traffic_class,
                flow_label: args.flow_label,
                encap_limit: args.encap_limit,
                pmtudisc: args.pmtudisc,
                link,
            };
            Ok(VirtualInterfaceBuild {
                name: args.name.clone(),
                admin_up: args.up,
                kind: VirtualInterfaceKind::Ip6Tnl(config),
            })
        }
        VirtualInterfaceCreateKind::Vlan(args) => {
            let base = resolve_optional_link(client, args.interface.as_deref())?;
            let config = VlanConfig {
                base_ifindex: base,
                vlan_id: args.vlan_id,
            };
            Ok(VirtualInterfaceBuild {
                name: args.name.clone(),
                admin_up: args.up,
                kind: VirtualInterfaceKind::Vlan(config),
            })
        }
    }
}

fn resolve_optional_link(client: &RtnlClient, name: Option<&str>) -> io::Result<Option<u32>> {
    match name {
        Some(dev) => Ok(Some(client.link().interface_get_by_name(dev)?.if_id)),
        None => Ok(None),
    }
}
#[derive(Subcommand)]
enum Command {
    /// List interfaces (optionally filtered by name)
    List {
        /// Interface name to filter on
        #[arg(short, long)]
        interface: Option<String>,
    },
    /// Show a single interface by name
    Show {
        /// Interface name
        interface: String,
    },
    /// Set operational state up/down
    SetState {
        /// Interface name
        interface: String,
        /// New state
        #[arg(value_parser = parse_bool_flag)]
        up: bool,
    },
    /// Enable or disable promiscuous mode
    SetPromisc {
        /// Interface name
        interface: String,
        #[arg(value_parser = parse_bool_flag)]
        enable: bool,
    },
    /// Enable or disable all-multicast mode
    SetAllMulticast {
        /// Interface name
        interface: String,
        #[arg(value_parser = parse_bool_flag)]
        enable: bool,
    },
    /// Enable or disable ARP
    SetArp {
        /// Interface name
        interface: String,
        #[arg(value_parser = parse_bool_flag)]
        enable: bool,
    },
    /// Set the interface MTU
    SetMtu {
        /// Interface name
        interface: String,
        /// MTU value
        mtu: u32,
    },
    /// Get the interface MTU
    GetMtu {
        /// Interface name
        interface: String,
    },
    /// Set the interface MAC address
    SetMac {
        /// Interface name
        interface: String,
        /// MAC address in hex colon notation
        mac: String,
    },
    /// Rename an interface
    Rename {
        /// Current interface name
        interface: String,
        /// New interface name
        new_name: String,
    },
    /// Manage virtual interfaces (tunnels, VLANs)
    VirtualInterface {
        #[command(subcommand)]
        command: VirtualInterfaceCommand,
    },
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let client = RtnlClient::new();
    match cli.command {
        Command::List { interface } => run_list(&client, interface.as_deref()),
        Command::Show { interface } => run_show(&client, &interface),
        Command::SetState { interface, up } => run_set_state(&client, &interface, up),
        Command::SetPromisc { interface, enable } => run_set_promisc(&client, &interface, enable),
        Command::SetAllMulticast { interface, enable } => {
            run_set_all_multicast(&client, &interface, enable)
        }
        Command::SetArp { interface, enable } => run_set_arp(&client, &interface, enable),
        Command::SetMtu { interface, mtu } => run_set_mtu(&client, &interface, mtu),
        Command::GetMtu { interface } => run_get_mtu(&client, &interface),
        Command::SetMac { interface, mac } => run_set_mac(&client, &interface, &mac),
        Command::Rename {
            interface,
            new_name,
        } => run_rename(&client, &interface, &new_name),
        Command::VirtualInterface { command } => run_virtual_interface(&client, command),
    }
}

fn run_list(client: &RtnlClient, interface: Option<&str>) -> io::Result<()> {
    let link_client = client.link();
    let interfaces = match interface {
        Some(name) => vec![link_client.interface_get_by_name(name)?],
        None => link_client.interface_list()?,
    };

    if interfaces.is_empty() {
        println!("No interfaces found");
        return Ok(());
    }

    for iface in interfaces {
        println!("{}: {}", iface.if_id, iface.if_name);
    }
    Ok(())
}

fn run_show(client: &RtnlClient, interface: &str) -> io::Result<()> {
    let link_client = client.link();
    let iface = link_client.interface_get_by_name(interface)?;

    println!("Interface {}:", iface.if_name);
    println!("  Index: {}", iface.if_id);

    match link_client.mac_addr_get(iface.if_id)? {
        Some(mac) => println!("  MAC: {}", mac),
        None => println!("  MAC: (unknown)"),
    }

    match link_client.mtu_get(iface.if_id) {
        Ok(mtu) => println!("  MTU: {}", mtu),
        Err(err) => println!("  MTU: failed to query ({})", err),
    }

    Ok(())
}

fn run_set_state(client: &RtnlClient, interface: &str, up: bool) -> io::Result<()> {
    let link_client = client.link();
    let iface = link_client.interface_get_by_name(interface)?;
    if up {
        link_client.interface_set_up(iface.if_id)?;
        println!("Interface {} set up", iface.if_name);
    } else {
        link_client.interface_set_down(iface.if_id)?;
        println!("Interface {} set down", iface.if_name);
    }
    Ok(())
}

fn run_set_promisc(client: &RtnlClient, interface: &str, enable: bool) -> io::Result<()> {
    let link_client = client.link();
    let iface = link_client.interface_get_by_name(interface)?;
    link_client.interface_set_promiscuous(iface.if_id, enable)?;
    println!(
        "Interface {} promiscuous mode {}",
        iface.if_name,
        if enable { "enabled" } else { "disabled" }
    );
    Ok(())
}

fn run_set_all_multicast(client: &RtnlClient, interface: &str, enable: bool) -> io::Result<()> {
    let link_client = client.link();
    let iface = link_client.interface_get_by_name(interface)?;
    link_client.interface_set_all_multicast(iface.if_id, enable)?;
    println!(
        "Interface {} all-multicast {}",
        iface.if_name,
        if enable { "enabled" } else { "disabled" }
    );
    Ok(())
}

fn run_set_arp(client: &RtnlClient, interface: &str, enable: bool) -> io::Result<()> {
    let link_client = client.link();
    let iface = link_client.interface_get_by_name(interface)?;
    link_client.interface_set_arp(iface.if_id, enable)?;
    println!(
        "Interface {} ARP {}",
        iface.if_name,
        if enable { "enabled" } else { "disabled" }
    );
    Ok(())
}

fn run_set_mtu(client: &RtnlClient, interface: &str, mtu: u32) -> io::Result<()> {
    let link_client = client.link();
    let iface = link_client.interface_get_by_name(interface)?;
    link_client.interface_set_mtu(iface.if_id, mtu)?;
    println!("Interface {} MTU set to {}", iface.if_name, mtu);
    Ok(())
}

fn run_get_mtu(client: &RtnlClient, interface: &str) -> io::Result<()> {
    let link_client = client.link();
    let iface = link_client.interface_get_by_name(interface)?;
    let mtu = link_client.mtu_get(iface.if_id)?;
    println!("Interface {} MTU: {}", iface.if_name, mtu);
    Ok(())
}

fn run_set_mac(client: &RtnlClient, interface: &str, mac: &str) -> io::Result<()> {
    let mac_addr = parse_mac(mac)?;
    let link_client = client.link();
    let iface = link_client.interface_get_by_name(interface)?;
    link_client.mac_addr_set(iface.if_id, mac_addr)?;
    println!("Interface {} MAC set to {}", iface.if_name, mac_addr);
    Ok(())
}

fn run_rename(client: &RtnlClient, interface: &str, new_name: &str) -> io::Result<()> {
    let link_client = client.link();
    let iface = link_client.interface_get_by_name(interface)?;
    link_client.interface_rename(iface.if_id, new_name)?;
    println!("Interface {} renamed to {}", iface.if_name, new_name);
    Ok(())
}

fn run_virtual_interface(client: &RtnlClient, command: VirtualInterfaceCommand) -> io::Result<()> {
    let vif_client = client.virtual_interface();
    match command {
        VirtualInterfaceCommand::Create(args) => {
            let VirtualInterfaceBuild {
                name,
                admin_up,
                kind,
            } = build_virtual_interface_kind(client, args.kind)?;
            let name = name.ok_or_else(|| {
                io::Error::other("--name is required for virtual-interface creation")
            })?;
            validate_virtual_interface_create(&kind)?;
            let spec = VirtualInterfaceSpec {
                name: name.clone(),
                admin_up: admin_up.unwrap_or(true),
                kind,
            };
            vif_client.create(spec)?;
            println!("Created virtual interface {}", name);
        }
        VirtualInterfaceCommand::Configure(args) => {
            let index = args.target.resolve_index(client)?;
            let VirtualInterfaceBuild {
                name: _,
                admin_up,
                kind,
            } = build_virtual_interface_kind(client, args.kind)?;
            let update = VirtualInterfaceUpdate {
                if_id: index,
                new_name: args.new_name.clone(),
                admin_up: args.admin_up.or(admin_up),
                kind,
            };
            vif_client.configure(update)?;
            println!("Configured virtual interface {}", index);
        }
        VirtualInterfaceCommand::Delete(args) => {
            let delete = args.target.to_delete()?;
            vif_client.delete(delete)?;
            println!("Virtual interface deleted");
        }
    }
    Ok(())
}

fn validate_virtual_interface_create(kind: &VirtualInterfaceKind) -> io::Result<()> {
    match kind {
        VirtualInterfaceKind::Vlan(cfg) => {
            if cfg.base_ifindex.is_none() {
                return Err(io::Error::other(
                    "--interface is required for VLAN virtual interfaces",
                ));
            }
            if cfg.vlan_id.is_none() {
                return Err(io::Error::other(
                    "--vlan-id is required for VLAN virtual interfaces",
                ));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn parse_mac(s: &str) -> io::Result<MacAddr> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 6 {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "Invalid MAC address",
        ));
    }

    let mut bytes = [0u8; 6];
    for (i, part) in parts.iter().enumerate() {
        bytes[i] = u8::from_str_radix(part, 16).map_err(|err| {
            io::Error::new(
                ErrorKind::InvalidInput,
                format!("Invalid MAC segment: {}", err),
            )
        })?;
    }
    Ok(MacAddr::new(bytes))
}

fn parse_bool_flag(value: &str) -> Result<bool, String> {
    match value.to_lowercase().as_str() {
        "up" | "enable" | "enabled" | "true" | "on" => Ok(true),
        "down" | "disable" | "disabled" | "false" | "off" => Ok(false),
        _ => Err(format!(
            "Cannot parse boolean flag from '{}': use up/down, on/off, true/false",
            value
        )),
    }
}
