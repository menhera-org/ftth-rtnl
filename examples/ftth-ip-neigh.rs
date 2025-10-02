use std::io;
use std::net::IpAddr;

use clap::{Args, Parser, Subcommand, ValueEnum};
use ftth_rtnl::{NeighborDelete, NeighborEntry, NeighbourFlags, NeighbourState, RtnlClient};

#[derive(Parser)]
#[command(author, version, about = "Manage neighbour entries with ftth-rtnl", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Add a neighbour entry
    Add(NeighbourArgs),
    /// Change an existing neighbour entry
    Change(NeighbourArgs),
    /// Delete a neighbour entry
    Delete(NeighbourDeleteArgs),
}

#[derive(Args, Clone)]
struct NeighbourArgs {
    /// Destination IP address
    destination: IpAddr,
    /// Interface name ( preferred )
    #[arg(long)]
    dev: Option<String>,
    /// Interface index
    #[arg(long, conflicts_with = "dev")]
    if_id: Option<u32>,
    /// Link layer address (e.g. 00:11:22:33:44:55)
    #[arg(long)]
    lladdr: Option<String>,
    /// Neighbour state
    #[arg(long, value_enum)]
    state: Option<StateArg>,
    /// Treat entry as router
    #[arg(long)]
    router: bool,
    /// Treat entry as proxy
    #[arg(long)]
    proxy: bool,
    /// Mark entry as permanent/sticky
    #[arg(long)]
    sticky: bool,
}

#[derive(Args, Clone)]
struct NeighbourDeleteArgs {
    destination: IpAddr,
    #[arg(long)]
    dev: Option<String>,
    #[arg(long, conflicts_with = "dev")]
    if_id: Option<u32>,
    #[arg(long)]
    lladdr: Option<String>,
    #[arg(long, value_enum)]
    state: Option<StateArg>,
    #[arg(long)]
    router: bool,
    #[arg(long)]
    proxy: bool,
    #[arg(long)]
    sticky: bool,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum StateArg {
    Incomplete,
    Reachable,
    Stale,
    Delay,
    Probe,
    Failed,
    Noarp,
    Permanent,
    None,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let client = RtnlClient::new();

    match cli.command {
        Command::Add(args) => run_add(&client, args),
        Command::Change(args) => run_change(&client, args),
        Command::Delete(args) => run_delete(&client, args),
    }
}

fn run_add(client: &RtnlClient, args: NeighbourArgs) -> io::Result<()> {
    let entry = build_neighbor_entry(client, args)?;
    client.neighbor().add(entry)?;
    println!("Neighbour entry added");
    Ok(())
}

fn run_change(client: &RtnlClient, args: NeighbourArgs) -> io::Result<()> {
    let entry = build_neighbor_entry(client, args)?;
    client.neighbor().change(entry)?;
    println!("Neighbour entry updated");
    Ok(())
}

fn run_delete(client: &RtnlClient, args: NeighbourDeleteArgs) -> io::Result<()> {
    let entry = build_neighbor_delete(client, args)?;
    client.neighbor().delete(entry)?;
    println!("Neighbour entry deleted");
    Ok(())
}

fn build_neighbor_entry(client: &RtnlClient, args: NeighbourArgs) -> io::Result<NeighborEntry> {
    let if_id = resolve_interface(client, args.if_id, args.dev)?;
    Ok(NeighborEntry {
        if_id,
        destination: args.destination,
        link_address: parse_lladdr(args.lladdr.as_deref())?,
        state: args.state.map(StateArg::into_state),
        flags: build_flags(args.router, args.proxy, args.sticky),
    })
}

fn build_neighbor_delete(
    client: &RtnlClient,
    args: NeighbourDeleteArgs,
) -> io::Result<NeighborDelete> {
    let if_id = resolve_interface(client, args.if_id, args.dev)?;
    Ok(NeighborDelete {
        if_id,
        destination: args.destination,
        link_address: parse_lladdr(args.lladdr.as_deref())?,
        state: args.state.map(StateArg::into_state),
        flags: build_flags(args.router, args.proxy, args.sticky),
    })
}

fn resolve_interface(
    client: &RtnlClient,
    if_id: Option<u32>,
    dev: Option<String>,
) -> io::Result<u32> {
    if let Some(index) = if_id {
        Ok(index)
    } else if let Some(name) = dev {
        Ok(client.link().interface_get_by_name(&name)?.if_id)
    } else {
        Err(io::Error::other("Specify either --dev or --if-id"))
    }
}

fn parse_lladdr(value: Option<&str>) -> io::Result<Option<Vec<u8>>> {
    match value {
        Some(s) => {
            let mut bytes = Vec::new();
            for part in s.split(':') {
                if part.is_empty() {
                    return Err(io::Error::other("Invalid link-layer address"));
                }
                let byte = u8::from_str_radix(part, 16)
                    .map_err(|_| io::Error::other("Invalid link-layer address component"))?;
                bytes.push(byte);
            }
            Ok(Some(bytes))
        }
        None => Ok(None),
    }
}

fn build_flags(router: bool, proxy: bool, sticky: bool) -> Option<NeighbourFlags> {
    let mut flags = NeighbourFlags::default();
    if router {
        flags.insert(NeighbourFlags::Router);
    }
    if proxy {
        flags.insert(NeighbourFlags::Proxy);
    }
    if sticky {
        flags.insert(NeighbourFlags::Sticky);
    }
    if flags.is_empty() { None } else { Some(flags) }
}

impl StateArg {
    fn into_state(self) -> NeighbourState {
        match self {
            StateArg::Incomplete => NeighbourState::Incomplete,
            StateArg::Reachable => NeighbourState::Reachable,
            StateArg::Stale => NeighbourState::Stale,
            StateArg::Delay => NeighbourState::Delay,
            StateArg::Probe => NeighbourState::Probe,
            StateArg::Failed => NeighbourState::Failed,
            StateArg::Noarp => NeighbourState::Noarp,
            StateArg::Permanent => NeighbourState::Permanent,
            StateArg::None => NeighbourState::None,
        }
    }
}
