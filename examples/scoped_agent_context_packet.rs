#[allow(dead_code)]
mod packet {
    include!("agent_context_packet.rs");
    include!("support/scoped_agent_context_packet_impl.rs");
    include!("support/scoped_selected_evidence_impl.rs");
}

fn main() {
    if let Err(error) = packet::run_scoped_with_optional_protection() {
        eprintln!("scoped-agent-context-packet: {error}");
        std::process::exit(1);
    }
}
