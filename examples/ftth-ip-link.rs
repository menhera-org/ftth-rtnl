use std::io::{self, ErrorKind};

use clap::{Parser, Subcommand};
use ftth_rtnl::{RtnlClient, link::MacAddr};

#[derive(Parser)]
#[command(author, version, about = "Minimal link management utility built on ftth-rtnl", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
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
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let client = RtnlClient::new();
    match cli.command {
        Command::List { interface } => run_list(&client, interface.as_deref()),
        Command::Show { interface } => run_show(&client, &interface),
        Command::SetState { interface, up } => run_set_state(&client, &interface, up),
        Command::SetPromisc { interface, enable } => run_set_promisc(&client, &interface, enable),
        Command::SetArp { interface, enable } => run_set_arp(&client, &interface, enable),
        Command::SetMtu { interface, mtu } => run_set_mtu(&client, &interface, mtu),
        Command::GetMtu { interface } => run_get_mtu(&client, &interface),
        Command::SetMac { interface, mac } => run_set_mac(&client, &interface, &mac),
        Command::Rename {
            interface,
            new_name,
        } => run_rename(&client, &interface, &new_name),
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
