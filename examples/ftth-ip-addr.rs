use std::io::{self, ErrorKind};

use clap::{Parser, Subcommand, ValueEnum};
use ftth_rtnl::{IpNet, RtnlClient};

#[derive(Parser)]
#[command(author, version, about = "Minimal IP address management utility built on ftth-rtnl", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List IP addresses on interfaces
    List {
        /// Restrict the output to a single interface by name
        #[arg(short, long)]
        interface: Option<String>,
        /// Limit the address family in the output
        #[arg(short = 'f', long, value_enum, default_value_t = AddressFamily::All)]
        family: AddressFamily,
    },
    /// Add an IP address (in CIDR notation) to an interface
    Add {
        /// Interface name (for example, eth0)
        interface: String,
        /// Address in CIDR notation (for example, 192.0.2.1/24 or 2001:db8::1/64)
        prefix: String,
    },
    /// Delete an IP address (in CIDR notation) from an interface
    Del {
        /// Interface name (for example, eth0)
        interface: String,
        /// Address in CIDR notation (for example, 192.0.2.1/24 or 2001:db8::1/64)
        prefix: String,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum AddressFamily {
    All,
    Ipv4,
    Ipv6,
}

impl AddressFamily {
    fn includes_ipv4(self) -> bool {
        matches!(self, AddressFamily::All | AddressFamily::Ipv4)
    }

    fn includes_ipv6(self) -> bool {
        matches!(self, AddressFamily::All | AddressFamily::Ipv6)
    }
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let client = RtnlClient::new();

    match cli.command {
        Command::List { interface, family } => run_list(&client, interface.as_deref(), family),
        Command::Add { interface, prefix } => run_add(&client, &interface, &prefix),
        Command::Del { interface, prefix } => run_del(&client, &interface, &prefix),
    }
}

fn run_list(client: &RtnlClient, interface: Option<&str>, family: AddressFamily) -> io::Result<()> {
    let link_client = client.link();
    let addr_client = client.address();

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

        if family.includes_ipv4() {
            let addrs = addr_client.ipv4_addrs_get(Some(iface.if_id))?;
            if addrs.is_empty() {
                println!("  IPv4: (none)");
            } else {
                for addr in addrs {
                    println!("  IPv4: {}", addr);
                }
            }
        }

        if family.includes_ipv6() {
            let addrs = addr_client.ipv6_addrs_get(Some(iface.if_id))?;
            if addrs.is_empty() {
                println!("  IPv6: (none)");
            } else {
                for addr in addrs {
                    println!("  IPv6: {}", addr);
                }
            }
        }

        println!();
    }

    Ok(())
}

fn run_add(client: &RtnlClient, interface: &str, prefix: &str) -> io::Result<()> {
    let net = parse_ip_net(prefix)?;
    let link_client = client.link();
    let addr_client = client.address();
    let if_id = link_client.interface_get_by_name(interface)?.if_id;

    match net {
        IpNet::V4(net) => {
            addr_client.ipv4_addr_set(if_id, net)?;
            println!("Added {} to {}", net, interface);
        }
        IpNet::V6(net) => {
            addr_client.ipv6_addr_set(if_id, net)?;
            println!("Added {} to {}", net, interface);
        }
    }

    Ok(())
}

fn run_del(client: &RtnlClient, interface: &str, prefix: &str) -> io::Result<()> {
    let net = parse_ip_net(prefix)?;
    let link_client = client.link();
    let addr_client = client.address();
    let if_id = link_client.interface_get_by_name(interface)?.if_id;

    match net {
        IpNet::V4(net) => {
            addr_client.ipv4_addr_del(if_id, net)?;
            println!("Deleted {} from {}", net, interface);
        }
        IpNet::V6(net) => {
            addr_client.ipv6_addr_del(if_id, net)?;
            println!("Deleted {} from {}", net, interface);
        }
    }

    Ok(())
}

fn parse_ip_net(s: &str) -> io::Result<IpNet> {
    s.parse::<IpNet>()
        .map_err(|err| io::Error::new(ErrorKind::InvalidInput, err))
}
