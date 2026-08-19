mod packet {
    include!("agent_context_packet.rs");
    include!("scoped_agent_context_packet_impl.rs");
}

fn main() {
    if let Err(error) = packet::run_scoped() {
        eprintln!("scoped-agent-context-packet: {error}");
        std::process::exit(1);
    }
}
