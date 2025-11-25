# Framework Control - Native GUI

A lightweight, native Windows application for controlling Framework laptops with grid-based fan curve editor and full BIOS integration.

![Framework Control](https://img.shields.io/badge/Framework-Laptop-orange)
![Rust](https://img.shields.io/badge/Rust-native-red)
![License](https://img.shields.io/badge/license-MIT-blue)

## 🚀 Features

### Grid-Based Fan Curve Editor ✨
- **Three Control Modes:**
  - 🔵 Auto - System controlled fan speed
  - 🟡 Manual - Fixed fan speed (0-100%)
  - 🟢 Curve - Temperature-based automatic adjustment

- **Interactive Fan Curve:**
  - Edit temperature/duty points in real-time grid
  - Add/Remove curve points (up to 10)
  - Linear interpolation between points
  - Continuous application every 5 seconds

### Real-Time Monitoring 📊
- CPU/GPU temperature display with color-coded warnings
- Fan RPM monitoring
- Battery status and charging state
- Power consumption metrics

### Power Management ⚡
- TDP control (5-28W)
- Thermal limit adjustment (60-100°C)
- AMD RyzenAdj integration
- Custom power profiles

### Battery Settings 🔋
- Charge limit control (50-100%)
- Framework EC integration
- 80% recommended for longevity

## 📦 Installation

### Requirements
- Windows 10/11
- [Rust](https://rustup.rs/) (latest stable)
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022)
- [framework_tool](https://github.com/FrameworkComputer/framework-system) (optional but recommended)
- [RyzenAdj](https://github.com/FlyGoat/RyzenAdj) (optional, for AMD CPUs)

### Quick Start

```powershell
# Clone the repository
git clone https://github.com/Alexxiconify/framework-control.git
cd framework-control

# Build (debug - fast)
cd service
cargo build

# Run
.\target\debug\framework-control.exe
```

### Fast Build Script

```powershell
# Quick debug build
.\fast_build.ps1

# Optimized release build
.\fast_build.ps1 -Release

# Build and run
.\fast_build.ps1 -Run
```

## 🎮 Usage

### Fan Control

**Manual Mode:**
1. Select "Manual" radio button
2. Adjust slider (0-100%)
3. Click "⚡ Apply"
4. Fan speed changes immediately

**Curve Mode:**
1. Select "Curve" radio button
2. Edit points in the grid:
   - Temperature (20-100°C)
   - Fan duty (0-100%)
3. Click "⚡ Apply Curve"
4. Background task monitors temps every 5 seconds
5. Fan speed adjusts automatically

**Auto Mode:**
- Click "🔄 Reset Auto"
- Returns control to BIOS

### Power Management

1. Check "Custom Limits"
2. Adjust TDP (5-28W)
3. Adjust Thermal Limit (60-100°C)
4. Click "⚡ Apply"

### Battery Settings

1. Check "Charge Limit"
2. Set max charge (50-100%)
3. Click "🔋 Apply"

## 🔧 Architecture

### Technology Stack
- **GUI:** egui (immediate mode GUI)
- **Backend:** Tokio async runtime
- **BIOS Integration:** framework_tool CLI
- **Power Management:** RyzenAdj (AMD)

### Project Structure
```
framework-control/
├── service/
│   ├── src/
│   │   ├── main.rs           # GUI + application logic
│   │   ├── config.rs         # Configuration management
│   │   ├── types.rs          # Data structures
│   │   ├── cli/              # CLI tool wrappers
│   │   │   ├── framework_tool.rs
│   │   │   └── ryzen_adj.rs
│   │   ├── tasks/            # Background tasks
│   │   └── utils/            # Utilities
│   └── Cargo.toml
├── fast_build.ps1            # Build script
└── README.md
```

## 📊 Performance

- **Binary Size:** 6-7 MB (release)
- **Memory Usage:** ~100 MB
- **CPU Usage:** < 1% (idle)
- **Build Time:** ~1 min (debug), ~5 min (release)
- **Startup Time:** < 1 second

## 🎯 Fan Curve Algorithm

The fan curve uses linear interpolation between points:

```rust
// Example: temp = 65°C between (60°C, 40%) and (70°C, 60%)
ratio = (65 - 60) / (70 - 60) = 0.5
duty = 40 + (60 - 40) * 0.5 = 50%
```

Background task applies curve every 5 seconds:
1. Read maximum CPU temperature
2. Interpolate fan duty from curve
3. Apply via `framework_tool --fan-duty X`

## 🛠️ Development

### Build Options

```powershell
# Debug build (fast, larger binary)
cargo build

# Release build (optimized, smaller binary)
cargo build --release

# Check without building
cargo check

# Run tests
cargo test
```

### Dependencies

Core (9 essential packages):
- `tokio` - Async runtime
- `serde`/`serde_json` - Serialization
- `tracing` - Logging
- `sysinfo` - System info
- `reqwest` - HTTP client
- `zip` - Archive handling
- `eframe`/`egui`/`egui_plot` - Native GUI

## 📝 Configuration

Create a `config.json` in the application directory:

```json
{
  "fan": {
    "mode": "curve",
    "curve": [
      [40, 20],
      [50, 30],
      [60, 40],
      [70, 60],
      [80, 80],
      [90, 100]
    ]
  },
  "power": {
    "tdp_watts": 15,
    "thermal_limit": 80
  },
  "battery": {
    "charge_limit": 80
  }
}
```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

### Areas for Improvement
- [ ] Visual curve preview with egui_plot
- [ ] Curve presets (Silent, Balanced, Performance)
- [ ] Save/load custom curves
- [ ] System tray mode
- [ ] Auto-start option
- [ ] Multi-language support

## 📄 License

MIT License - see LICENSE file for details

## 🙏 Acknowledgments

- [Framework Computer](https://frame.work/) - For making amazing laptops
- [framework_tool](https://github.com/FrameworkComputer/framework-system) - EC communication tool
- [egui](https://github.com/emilk/egui) - Immediate mode GUI library
- [RyzenAdj](https://github.com/FlyGoat/RyzenAdj) - AMD power management

## ⚠️ Disclaimer

This software controls hardware parameters. Use at your own risk. The author is not responsible for any hardware damage. Always monitor temperatures when using custom fan curves.

## 📞 Support

- Issues: [GitHub Issues](https://github.com/Alexxiconify/framework-control/issues)
- Framework Community: [community.frame.work](https://community.frame.work)

---

**Made with ❤️ for the Framework Community**