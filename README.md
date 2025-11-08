# mDNS Responder for Windows - Samba/SMB Advertisement

A modern **WIP** Rust application that implements a Bonjour/mDNS service for Windows, enabling automatic discovery and advertisement of Samba (SMB) file shares on your local network. Compatible with macOS, Linux, and other mDNS-enabled devices.

## ✨ Key Features

- **Pure Rust Implementation**: Zero C dependencies, no FFI complexity
- **Cross-Platform Discovery**: Works seamlessly with macOS, Linux, and iOS clients
- **Intelligent IP Detection**: Automatically detects local network IP, skips VPN/virtual adapters
- **Windows Service Integration**: Runs as a Windows service with automatic startup
- **SMB/Samba Advertisement**: Full RFC 6763 DNS-SD compliance with TXT records
- **JSON Configuration**: Simple, validated configuration file support
- **Graceful Shutdown**: Thread-safe shutdown with configurable timeout
- **Comprehensive Logging**: Structured logging with configurable levels

## 🚀 Quick Start

### 1. Build Release Binary

```powershell
cargo build --release
# Binary: target\release\mdns_responder.exe
```

### 2. Test Standalone Mode

```powershell
$env:RUST_LOG='info'
.\target\release\mdns_responder.exe run

# Expected output:
# [INFO] Initializing mDNS Responder Service...
# [INFO] Using configuration: ServiceConfig { ... }
# [INFO] Auto-detected local IP address: 192.168.1.11
# [INFO] Using hostname: windows-pc.local
# [INFO] Successfully registered Windows-Share on port 445 with IP 192.168.1.11
```

### 3. Verify Discovery on macOS

```bash
# On Mac
dns-sd -B _smb._tcp local
# Should see: Windows-Share._smb._tcp.local.

# Resolve hostname
dns-sd -G v4 windows-pc.local
# Should see: windows-pc.local -> 192.168.1.11

# View service details
dns-sd -L "Windows-Share" _smb._tcp local
```

### 4. Install as Windows Service

```powershell
# Run as Administrator
.\target\release\mdns_responder.exe install
net start MDNSResponder

# Verify status
sc query MDNSResponder
```

## ⚙️ Service Management

| Command | Purpose |
|---------|---------|
| `net start MDNSResponder` | Start service |
| `net stop MDNSResponder` | Stop service |
| `sc query MDNSResponder` | Check status |
| `sc delete MDNSResponder` | Remove service (alt) |
| `.\target\release\mdns_responder.exe uninstall` | Remove service |

## 📋 Configuration

Configuration file: `C:\ProgramData\MDNSResponder\config.json`

```json
{
  "service_name": "_smb._tcp.local.",
  "instance_name": "Windows-Share",
  "port": 445,
  "hostname": "windows-pc.local",
  "workgroup": "WORKGROUP",
  "description": "Windows SMB Share via mDNS",
  "bind_address": "192.168.1.11",
  "shares": [
    {
      "name": "Documents",
      "path": "C:\\Users\\Public\\Documents",
      "comment": "Public documents"
    }
  ]
}
```

### Optional: Manual IP Binding

If auto-detection fails (e.g., VPN conflicts), add `bind_address`:

```json
{
  ...
  "bind_address": "192.168.1.11"
}
```

## 🔒 Firewall Configuration

### Windows Firewall Setup

```powershell
# mDNS multicast (UDP 5353)
netsh advfirewall firewall add rule name="mDNS In" dir=in action=allow protocol=udp localport=5353
netsh advfirewall firewall add rule name="mDNS Out" dir=out action=allow protocol=udp remoteport=5353

# SMB protocol (TCP 445)
netsh advfirewall firewall add rule name="SMB" dir=in action=allow protocol=tcp localport=445
```

### Verify Ports

```powershell
netstat -an | findstr ":445"
netstat -an | findstr ":5353"
```

## 📊 Logging

```powershell
$env:RUST_LOG='info'  # or 'debug', 'trace'
.\target\release\mdns_responder.exe run
```

When running as service, check Windows Event Viewer:
- Event Viewer → Applications and Services Logs → Application
- Filter source: `MDNSResponder`

## 🔧 Troubleshooting

### Service won't start
- ✅ Run Command Prompt/PowerShell as Administrator
- ✅ Check Windows Event Viewer for errors
- ✅ Verify firewall allows UDP 5353 & TCP 445
- ✅ Check for port conflicts: `netstat -ano | findstr ":5353"`

### macOS can't find Windows-Share
- ✅ Run `dns-sd -B _smb._tcp local` on Mac (should see service)
- ✅ Run `dns-sd -G v4 windows-pc.local` on Mac (should resolve IP)
- ✅ Ensure Windows is on same network segment (not VPN)
- ✅ Check hostname has `.local` suffix in config

### Auto-detection picks wrong IP (VPN/Virtual)
- ✅ Manually set `bind_address` in config.json
- ✅ Use your actual LAN IP (e.g., `192.168.x.x`)
- ✅ Restart service after config change

## 🏗️ Architecture

```
src/
├── lib.rs              # Module exports
├── config.rs          # Configuration management + validation
├── error.rs           # Unified error types with From traits
├── mdns_service.rs    # mDNS daemon (core logic)
├── discovery.rs       # Service discovery (debug-only)
└── windows_service.rs # Windows service integration
```

### Design Principles

- **Zero unsafe code** - Pure safe Rust
- **Error handling** - Custom `#[from]` traits eliminate boilerplate
- **Thread safety** - `Arc<Mutex>` pattern for graceful shutdown
- **Memory safety** - No manual memory management, Rust compiler ensures safety
- **Configuration validation** - All settings validated at load time

## 🎯 Development

### Run Tests

```powershell
cargo test --release
```

### Debug Mode

```powershell
cargo run -- discover  # (debug build only)
```

### Check Code Quality

```powershell
cargo clippy -- -D warnings
cargo fmt -- --check
```

## 📦 Version History

### 0.01a (Current)
- ✅ Core mDNS service registration
- ✅ Intelligent IP detection (skips VPN/virtual adapters)
- ✅ Windows service integration
- ✅ RFC 6763 compliant TXT records
- ✅ Graceful shutdown with Arc<Mutex>
- ✅ Configuration file support
- ✅ Cross-platform logging

### Planned

- [ ] Firewall auto-configuration
- [ ] Service conflict resolution (auto-rename)
- [ ] Configuration hot-reload
- [ ] Multi-adapter support
- [ ] WiX installer (.msi)
- [ ] Performance monitoring

## 📚 Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `mdns-sd` | 0.7.5 | mDNS/DNS-SD implementation |
| `ipconfig` | 0.3 | Network adapter detection |
| `windows` | 0.62 | Windows API bindings |
| `windows-service` | 0.7 | Service Control Manager |
| `tokio` | 1.48 | Async runtime |
| `serde/serde_json` | 1.0 | Configuration serialization |
| `log/env_logger` | 0.11 | Logging framework |
| `thiserror` | 1.0 | Error types |
| `ctrlc` | 3.4 | Signal handling |

## 📖 References

- [RFC 6763 - DNS-SD](https://tools.ietf.org/html/rfc6763)
- [RFC 6762 - mDNS](https://tools.ietf.org/html/rfc6762)
- [mdns-sd Crate](https://crates.io/crates/mdns-sd)
- [SMB Protocol Documentation](https://docs.microsoft.com/en-us/windows/win32/fileio/microsoft-smb-protocol-and-cifs-protocol-overview)

## 📄 License

MIT OR Apache-2.0

## 🤝 Contributing

Contributions welcome! Please:
1. Fork the repository
2. Create feature branch (`git checkout -b feature/amazing-feature`)
3. Commit changes (`git commit -m 'Add amazing feature'`)
4. Push branch (`git push origin feature/amazing-feature`)
5. Open Pull Request

---

**Status**: 🟢 WIP (0.01a)  
**Last Updated**: November 2025
