use std::collections::HashMap;
use std::io::{self, ErrorKind};
use std::net::{Ipv4Addr, Ipv6Addr};

use clap::{Args, Parser, Subcommand, ValueEnum};
use ftth_rtnl::{Ipv4Route, Ipv6Route, RtnlClient};
use ipnet::IpNet;

#[derive(Parser)]
#[command(author, version, about = "Manage IP routes with ftth-rtnl", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List routes
    List {
        #[arg(value_enum, default_value_t = RouteFamily::V4)]
        family: RouteFamily,
    },
    /// Add an IPv4 route
    Add4(RouteV4Args),
    /// Add an IPv6 route
    Add6(RouteV6Args),
    /// Delete an IPv4 route
    Del4(RouteV4DeleteArgs),
    /// Delete an IPv6 route
    Del6(RouteV6DeleteArgs),
    /// Lookup the selected IPv4 route
    Get4 { destination: Ipv4Addr },
    /// Lookup the selected IPv6 route
    Get6 { destination: Ipv6Addr },
}

#[derive(ValueEnum, Clone, Copy)]
enum RouteFamily {
    V4,
    V6,
}

#[derive(Args, Clone)]
struct RouteV4Args {
    /// Destination prefix in CIDR notation (e.g. 192.0.2.0/24)
    prefix: String,
    /// Gateway IPv4 address
    #[arg(long)]
    via: Option<Ipv4Addr>,
    /// Output interface name
    #[arg(long)]
    dev: Option<String>,
    /// Preferred source IPv4 address
    #[arg(long)]
    src: Option<Ipv4Addr>,
    /// Route metric
    #[arg(long)]
    metric: Option<u32>,
    /// Route table ID
    #[arg(long)]
    table: Option<u32>,
    /// Replace an existing route instead of adding a new one
    #[arg(long)]
    replace: bool,
}

#[derive(Args, Clone)]
struct RouteV6Args {
    /// Destination prefix in CIDR notation (e.g. 2001:db8::/32)
    prefix: String,
    /// Gateway IPv6 address
    #[arg(long)]
    via: Option<Ipv6Addr>,
    /// Output interface name
    #[arg(long)]
    dev: Option<String>,
    /// Preferred source IPv6 address
    #[arg(long)]
    src: Option<Ipv6Addr>,
    /// Route metric
    #[arg(long)]
    metric: Option<u32>,
    /// Route table ID
    #[arg(long)]
    table: Option<u32>,
    /// Replace an existing route instead of adding a new one
    #[arg(long)]
    replace: bool,
}

#[derive(Args, Clone)]
struct RouteV4DeleteArgs {
    prefix: String,
    #[arg(long)]
    via: Option<Ipv4Addr>,
    #[arg(long)]
    dev: Option<String>,
    #[arg(long)]
    table: Option<u32>,
}

#[derive(Args, Clone)]
struct RouteV6DeleteArgs {
    prefix: String,
    #[arg(long)]
    via: Option<Ipv6Addr>,
    #[arg(long)]
    dev: Option<String>,
    #[arg(long)]
    table: Option<u32>,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let client = RtnlClient::new();

    match cli.command {
        Command::List { family } => run_list(&client, family),
        Command::Add4(args) => run_add4(&client, args),
        Command::Add6(args) => run_add6(&client, args),
        Command::Del4(args) => run_del4(&client, args),
        Command::Del6(args) => run_del6(&client, args),
        Command::Get4 { destination } => run_get4(&client, destination),
        Command::Get6 { destination } => run_get6(&client, destination),
    }
}

fn run_list(client: &RtnlClient, family: RouteFamily) -> io::Result<()> {
    let link_map = build_interface_map(client)?;
    match family {
        RouteFamily::V4 => {
            for route in client.route().ipv4_route_list()? {
                print_ipv4_route(&route, &link_map)?;
            }
        }
        RouteFamily::V6 => {
            for route in client.route().ipv6_route_list()? {
                print_ipv6_route(&route, &link_map)?;
            }
        }
    }
    Ok(())
}

fn run_add4(client: &RtnlClient, args: RouteV4Args) -> io::Result<()> {
    let route = build_ipv4_route(
        client,
        &args.prefix,
        args.via,
        args.dev,
        args.src,
        args.metric,
        args.table,
    )?;
    if args.replace {
        client.route().ipv4_route_replace(route)?;
        println!("IPv4 route replaced");
    } else {
        client.route().ipv4_route_add(route)?;
        println!("IPv4 route added");
    }
    Ok(())
}

fn run_add6(client: &RtnlClient, args: RouteV6Args) -> io::Result<()> {
    let route = build_ipv6_route(
        client,
        &args.prefix,
        args.via,
        args.dev,
        args.src,
        args.metric,
        args.table,
    )?;
    if args.replace {
        client.route().ipv6_route_replace(route)?;
        println!("IPv6 route replaced");
    } else {
        client.route().ipv6_route_add(route)?;
        println!("IPv6 route added");
    }
    Ok(())
}

fn run_del4(client: &RtnlClient, args: RouteV4DeleteArgs) -> io::Result<()> {
    let route = build_ipv4_route(
        client,
        &args.prefix,
        args.via,
        args.dev,
        None,
        None,
        args.table,
    )?;
    client.route().ipv4_route_del(route)?;
    println!("IPv4 route deleted");
    Ok(())
}

fn run_del6(client: &RtnlClient, args: RouteV6DeleteArgs) -> io::Result<()> {
    let route = build_ipv6_route(
        client,
        &args.prefix,
        args.via,
        args.dev,
        None,
        None,
        args.table,
    )?;
    client.route().ipv6_route_del(route)?;
    println!("IPv6 route deleted");
    Ok(())
}

fn run_get4(client: &RtnlClient, destination: Ipv4Addr) -> io::Result<()> {
    let route = client.route().ipv4_route_get(destination)?;
    let link_map = build_interface_map(client)?;
    print_ipv4_route(&route, &link_map)
}

fn run_get6(client: &RtnlClient, destination: Ipv6Addr) -> io::Result<()> {
    let route = client.route().ipv6_route_get(destination)?;
    let link_map = build_interface_map(client)?;
    print_ipv6_route(&route, &link_map)
}

fn build_ipv4_route(
    client: &RtnlClient,
    prefix: &str,
    gateway: Option<Ipv4Addr>,
    dev: Option<String>,
    source: Option<Ipv4Addr>,
    metric: Option<u32>,
    table: Option<u32>,
) -> io::Result<Ipv4Route> {
    let net = match prefix
        .parse::<IpNet>()
        .map_err(|e| io::Error::new(ErrorKind::InvalidInput, e))?
    {
        IpNet::V4(net) => net,
        IpNet::V6(_) => {
            return Err(io::Error::other("Expected IPv4 prefix"));
        }
    };
    let if_id = resolve_interface(client, dev)?;
    Ok(Ipv4Route {
        if_id,
        gateway,
        source,
        metric,
        table,
        route: net,
    })
}

fn build_ipv6_route(
    client: &RtnlClient,
    prefix: &str,
    gateway: Option<Ipv6Addr>,
    dev: Option<String>,
    source: Option<Ipv6Addr>,
    metric: Option<u32>,
    table: Option<u32>,
) -> io::Result<Ipv6Route> {
    let net = match prefix
        .parse::<IpNet>()
        .map_err(|e| io::Error::new(ErrorKind::InvalidInput, e))?
    {
        IpNet::V6(net) => net,
        IpNet::V4(_) => {
            return Err(io::Error::other("Expected IPv6 prefix"));
        }
    };
    let if_id = resolve_interface(client, dev)?;
    Ok(Ipv6Route {
        if_id,
        gateway,
        source,
        metric,
        table,
        route: net,
    })
}

fn resolve_interface(client: &RtnlClient, name: Option<String>) -> io::Result<Option<u32>> {
    match name {
        Some(dev) => Ok(Some(client.link().interface_get_by_name(&dev)?.if_id)),
        None => Ok(None),
    }
}

fn build_interface_map(client: &RtnlClient) -> io::Result<HashMap<u32, String>> {
    let mut map = HashMap::new();
    for iface in client.link().interface_list()? {
        map.insert(iface.if_id, iface.if_name);
    }
    Ok(map)
}

fn print_ipv4_route(route: &Ipv4Route, links: &HashMap<u32, String>) -> io::Result<()> {
    let dev = route.if_id.and_then(|id| links.get(&id).cloned());
    let via = route
        .gateway
        .map(|g| g.to_string())
        .unwrap_or_else(|| "direct".into());
    let dev_str = dev.unwrap_or_else(|| "-".into());
    let metric = route.metric.map_or("-".into(), |m| m.to_string());
    let table = route.table.map_or("main".into(), |t| t.to_string());
    let source = route.source.map_or("-".into(), |s| s.to_string());
    println!(
        "{} via {} dev {} src {} metric {} table {}",
        route.route, via, dev_str, source, metric, table,
    );
    Ok(())
}

fn print_ipv6_route(route: &Ipv6Route, links: &HashMap<u32, String>) -> io::Result<()> {
    let dev = route.if_id.and_then(|id| links.get(&id).cloned());
    let via = route
        .gateway
        .map(|g| g.to_string())
        .unwrap_or_else(|| "direct".into());
    let dev_str = dev.unwrap_or_else(|| "-".into());
    let metric = route.metric.map_or("-".into(), |m| m.to_string());
    let table = route.table.map_or("main".into(), |t| t.to_string());
    let source = route.source.map_or("-".into(), |s| s.to_string());
    println!(
        "{} via {} dev {} src {} metric {} table {}",
        route.route, via, dev_str, source, metric, table,
    );
    Ok(())
}
