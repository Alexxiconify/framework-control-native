# Framework Control

A Windows service for controlling Framework laptop settings including fan curves, power limits, battery management, and system monitoring.

## Features

- **Fan Control**: Manual or curve-based fan speed control with calibration
- **Power Management**: TDP and thermal limit configuration for AC and battery
- **Battery Settings**: Charge limit and rate control
- **System Monitoring**: Real-time temperature, fan speed, and power telemetry
- **Auto-Updates**: Background update checking and installation
- **Web UI**: Modern web interface for all settings

## Prerequisites
### Quick Build (Debug)

```powershell
# Build service only
cd service
cargo build

# Build web UI
cd ../web
npm install
npm run build
```

### Full MSI Package

```powershell
# From the web directory
cd web
npm run build:msi -- --port 8090 --token YOUR_SECRET_TOKEN
```

This will:
1. Build the web UI
2. Compile the Rust service
3. Create an MSI installer in `service/target/wix/`

## Installation

### From MSI

1. Build or download the MSI package
2. Run the installer as Administrator
3. The service will be installed to `C:\Program Files\Framework Control\`
4. Desktop shortcuts will be created (optional during install)

### Manual Installation

1. Build the service:
   ```powershell
   cd service
   cargo build --release
   ```

2. Copy the executable to a permanent location
3. Create a `.env` file or set environment variables:
   ```
   FRAMEWORK_CONTROL_PORT=8090
   FRAMEWORK_CONTROL_TOKEN=your-secret-token-here
   FRAMEWORK_CONTROL_ALLOWED_ORIGINS=http://localhost:8090
   FRAMEWORK_CONTROL_UPDATE_REPO=owner/repo
   ```

4. Install as Windows Service (using WinSW or similar)

## Configuration

### Environment Variables

- `FRAMEWORK_CONTROL_PORT` (required): HTTP port for the service (e.g., 8090)
- `FRAMEWORK_CONTROL_TOKEN` (optional): Bearer token for authentication
- `FRAMEWORK_CONTROL_ALLOWED_ORIGINS`: Comma-separated CORS origins
- `FRAMEWORK_CONTROL_UPDATE_REPO`: GitHub repo for updates (format: owner/repo)
- `FRAMEWORK_CONTROL_CONFIG`: Custom config file path (default: `C:\ProgramData\FrameworkControl\config.json`)

### Configuration File

Settings are stored in: `C:\ProgramData\FrameworkControl\config.json`

The service automatically:
- Creates default configuration on first run
- Validates all settings
- Uses atomic writes to prevent corruption

## Usage

### Web Interface

1. Start the service (automatically starts if installed)
2. Open browser to `http://localhost:8090` (or your configured port)
3. Configure settings through the web UI

### API

The service exposes a REST API documented with OpenAPI/Swagger.

Access the API documentation at: `http://localhost:8090/api`

## Development

### Project Structure

```
framework-control-0.4.2/
├── service/           # Rust service
│   ├── src/
│   │   ├── main.rs   # Entry point
│   │   ├── routes.rs # API endpoints
│   │   ├── state.rs  # Application state
│   │   ├── config.rs # Configuration
│   │   ├── cli/      # CLI tool wrappers
│   │   ├── tasks/    # Background tasks
│   │   └── utils/    # Utilities
│   └── wix/          # MSI installer configuration
└── web/              # Svelte web UI
    ├── src/
    │   ├── components/
    │   └── lib/
    └── scripts/      # Build scripts
```

### Building for Development

```powershell
# Terminal 1: Watch and rebuild service
cd service
cargo watch -x run

# Terminal 2: Watch and rebuild web
cd web
npm run dev
```

### Running Tests

```powershell
# Service tests
cd service
cargo test

# Lint
cd web
npm run lint
```

### Generating OpenAPI Spec

```powershell
cd service
cargo run -- --generate-openapi
# Outputs to: ../web/openapi.json
```

## Troubleshooting

### Service Won't Start

1. Check environment variables are set correctly
2. Verify port is not in use: `netstat -ano | findstr :8090`
3. Check service logs in Event Viewer (Windows Logs > Application)

### CLI Tools Not Found

The service will automatically attempt to locate:
- `framework_tool.exe` (Framework's official CLI)
- `ryzenadj.exe` (AMD Ryzen power tuning)

Install these tools or use the web UI to install RyzenAdj.

### Configuration Issues

- Config file: `C:\ProgramData\FrameworkControl\config.json`
- Delete this file to reset to defaults
- Check file permissions if save fails

### Port Already in Use

Change the port in your environment variables:
```powershell
$env:FRAMEWORK_CONTROL_PORT = "8091"
```

## Security Notes

1. **Authentication**: Set `FRAMEWORK_CONTROL_TOKEN` to protect sensitive endpoints
2. **CORS**: Configure `FRAMEWORK_CONTROL_ALLOWED_ORIGINS` to restrict access
3. **Firewall**: The service binds to `127.0.0.1` (localhost only) by default
4. **Updates**: Auto-updates require valid GitHub repository configuration

## License

See [LICENSE](LICENSE) file for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test thoroughly
5. Submit a pull request

## Support

For issues, questions, or feature requests:
- Check existing GitHub issues
- Create a new issue with detailed information
- Include logs and system information