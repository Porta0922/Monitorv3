/// Never self-delete identity or unacknowledged events from an HTTP/remote command.
/// Removal is performed by the administrator-owned platform lifecycle tooling.
pub fn spawn_uninstall() -> Result<(), Box<dyn std::error::Error>> {
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported,
        "Automatic self-uninstall is disabled. Use the platform uninstall command; preserve identity and pending events.").into())
}
