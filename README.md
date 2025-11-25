# Framework Control - Native GUI

A lightweight native Windows application for controlling Framework laptops.

## 🚀 Features

- **Grid-Based Fan Curve Editor** - Custom temperature-based fan control
- **Real-Time Monitoring** - CPU/GPU temps, fan speeds, battery status
- **Power Management** - TDP and thermal limit control (AMD via RyzenAdj)
- **Battery Settings** - Charge limit control for battery longevity
- **Native Windows GUI** - No browser required, ~6 MB executable

## 📦 Installation

### Requirements
- Windows 10/11
- [Rust](https://rustup.rs/) (for building from source)
- [framework_tool](https://github.com/FrameworkComputer/framework-system) (optional)

### Quick Install

1. **Download or clone this repository**
2. **Run installer as Administrator:**
   ```powershell
   # Right-click PowerShell -> Run as Administrator
   cd path\to\framework-control-0.4.2
   .\install_system.ps1
   ```
3. **Follow prompts** - It will:
   - Build release version (~5 min)
   - Install to `C:\Program Files\FrameworkControl\`
   - Create Start Menu shortcut
   - Optionally create Desktop shortcut
   - Optionally add to system PATH

### Launch

- Press **Windows Key** → Type **"Framework Control"** → Click
- Or double-click Desktop shortcut
- Or run: `framework-control` (if added to PATH)

## 🎮 Usage

### Fan Control
- **Auto** - System managed (default)
- **Manual** - Fixed speed (0-100%)
- **Curve** - Temperature-based automatic adjustment with editable grid

### Power Management
- Set TDP (5-28W)
- Set thermal limit (60-100°C)
- Requires RyzenAdj for AMD CPUs

### Battery Settings
- Set charge limit (50-100%)
- Recommended: 80% for battery longevity

## 🔧 Building from Source

```powershell
# Navigate to service directory
cd service

# Build release version
cargo build --release

# Run
.\target\release\framework-control.exe
```

## 📊 Performance

- **Binary Size:** 6-7 MB (release)
- **Memory:** ~100 MB
- **CPU:** <1% idle
- **Startup:** <1 second

## 🗑️ Uninstall

```powershell
# Run as Administrator
.\install_system.ps1 -Uninstall
```

## 📄 License

MIT License - See LICENSE file

## 🙏 Credits

- [Framework Computer](https://frame.work/)
- [framework_tool](https://github.com/FrameworkComputer/framework-system)
- [egui](https://github.com/emilk/egui)
- [RyzenAdj](https://github.com/FlyGoat/RyzenAdj)

## ⚠️ Disclaimer

This software controls hardware. Use at your own risk. Monitor temperatures when using custom settings.

---

**Made for the Framework Community** ❤️