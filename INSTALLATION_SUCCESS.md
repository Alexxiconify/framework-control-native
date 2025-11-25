# Framework Control - Installation Complete ✓

## Installation Summary

The Framework Control application has been **successfully installed** on your system!

---

## What Was Fixed

### 1. **build_msi.ps1 Script Errors**
   - **Problem**: Unicode characters (✓, →, •) were causing PowerShell parse errors
   - **Solution**: Replaced all Unicode characters with ASCII-safe alternatives ([OK], ->, *)
   - **Result**: Script now runs without errors

### 2. **MSI Package Creation**
   - Successfully built MSI installer package
   - Location: `C:\Users\Taylor Allred\Documents\Files\projects\framework-control-native\output\FrameworkControl-0.4.2.msi`
   - Size: 9.8 MB

### 3. **Program Installation**
   - Installed to: `C:\Program Files (x86)\FrameworkControl\`
   - Executable: `framework-control.exe`
   - All files present and working

---

## Installation Verification ✓

| Component | Status | Details |
|-----------|--------|---------|
| MSI Package | ✓ Built | 9.8 MB installer created |
| Program Files | ✓ Installed | C:\Program Files (x86)\FrameworkControl\ |
| Service | ✓ Running | FrameworkControlService (Auto-start) |
| Start Menu | ✓ Created | Framework Control shortcut available |
| GUI Application | ✓ Working | Can be launched from Start Menu |

---

## How to Use

### Launch the GUI
1. Press **Windows Key**
2. Type: `Framework Control`
3. Click on the **Framework Control** shortcut
4. The GUI will open with fan control and system monitoring

### Service (Background)
- The **FrameworkControlService** runs automatically in the background
- Manages fan curves and power settings
- Starts automatically with Windows
- No user interaction needed

### Control Features
- **Fan Control**: Adjust fan curves for optimal cooling
- **Power Management**: Monitor and control power settings
- **Telemetry**: View CPU temperature, fan speeds, battery status
- **Profiles**: Save and load different configurations

---

## Technical Details

### Service Information
- **Name**: FrameworkControlService
- **Display Name**: Framework Control Service
- **Status**: Running
- **Start Type**: Automatic
- **Account**: LocalSystem
- **Description**: Manages Framework laptop fan curves and power settings

### File Locations
```
Installation Directory:
  C:\Program Files (x86)\FrameworkControl\
  
Main Executable:
  C:\Program Files (x86)\FrameworkControl\framework-control.exe
  
Start Menu Shortcut:
  C:\Users\[Username]\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Framework Control\
  
Configuration:
  Will be created on first run in user AppData
```

---

## Build Scripts

### To Rebuild the MSI:
```powershell
cd "C:\Users\Taylor Allred\Documents\Files\projects\framework-control-native"
.\build_msi.ps1
```

### To Clean and Rebuild:
```powershell
.\build_msi.ps1 -Clean
```

---

## Uninstallation

If you need to uninstall:

1. **Via Settings**:
   - Settings → Apps → Installed Apps
   - Find "Framework Control"
   - Click Uninstall

2. **Via Control Panel**:
   - Control Panel → Programs → Uninstall a program
   - Select "Framework Control"
   - Click Uninstall

3. **Via PowerShell**:
   ```powershell
   $app = Get-WmiObject -Class Win32_Product | Where-Object { $_.Name -eq "Framework Control" }
   $app.Uninstall()
   ```

The uninstaller will:
- Stop and remove the service
- Remove all program files
- Remove Start Menu shortcuts
- Clean up registry entries

---

## Troubleshooting

### Service Not Starting
```powershell
# Check service status
Get-Service FrameworkControlService

# Start service manually
Start-Service FrameworkControlService

# View service logs
Get-EventLog -LogName Application -Source "Framework Control" -Newest 10
```

### GUI Not Opening
```powershell
# Launch directly
& "C:\Program Files (x86)\FrameworkControl\framework-control.exe"

# Check if process is running
Get-Process framework-control -ErrorAction SilentlyContinue
```

### Reinstall
```powershell
# Uninstall first
msiexec /x "C:\Users\Taylor Allred\Documents\Files\projects\framework-control-native\output\FrameworkControl-0.4.2.msi" /qb

# Then reinstall
msiexec /i "C:\Users\Taylor Allred\Documents\Files\projects\framework-control-native\output\FrameworkControl-0.4.2.msi" /qb
```

---

## Success! 🎉

Your Framework Control application is now:
- ✓ Fully installed
- ✓ Service running in background
- ✓ Accessible from Start Menu
- ✓ Ready to use

**Enjoy your optimized Framework laptop control!**