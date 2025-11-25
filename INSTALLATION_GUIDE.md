# Framework Control - System Installation Guide

## ✅ Installation Complete!

Your Framework Control application is now installed as a system-wide program.

---

## 📍 Installation Details

### Installed Location:
```
C:\Program Files\FrameworkControl\
├── framework-control.exe    # Main application
├── README.md                 # Documentation
├── LICENSE                   # MIT License
└── config/
    └── config.json          # Default configuration
```

### Shortcuts Created:
- ✅ **Start Menu:** Press Windows key → Type "Framework Control"
- ✅ **Desktop:** (If you selected this option)

### System Integration:
- ✅ Added to system PATH (if selected)
- ✅ Can run from command line: `framework-control`

---

## 🚀 How to Launch

### Method 1: Start Menu (Recommended)
1. Press **Windows key**
2. Type: **"Framework Control"**
3. Click the application
4. Application opens!

### Method 2: Desktop Shortcut
- Double-click **"Framework Control"** icon on desktop

### Method 3: Command Line
```powershell
# From anywhere in terminal
framework-control

# Or direct path
"C:\Program Files\FrameworkControl\framework-control.exe"
```

### Method 4: Run Dialog
1. Press **Windows + R**
2. Type: `framework-control`
3. Press Enter

---

## ⚙️ Configuration

### Config File Location:
```
C:\Program Files\FrameworkControl\config\config.json
```

### Default Settings:
```json
{
  "fan": {
    "mode": "auto",
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

### To Edit Configuration:
1. Open as Administrator: Notepad or any text editor
2. Edit: `C:\Program Files\FrameworkControl\config\config.json`
3. Save changes
4. Restart Framework Control

---

## 🔧 Features Available

### Fan Control:
- **Auto Mode:** BIOS controlled (default)
- **Manual Mode:** Set fixed speed (0-100%)
- **Curve Mode:** Temperature-based automatic adjustment

### Power Management:
- TDP control (5-28W)
- Thermal limit (60-100°C)
- Requires RyzenAdj for AMD CPUs

### Battery Settings:
- Charge limit (50-100%)
- Recommended: 80% for battery longevity

### Monitoring:
- Real-time CPU/GPU temperatures
- Fan speeds (RPM)
- Battery status
- Power consumption

---

## 🔄 Auto-Start (Optional)

### To Launch at Windows Startup:

1. Press **Windows + R**
2. Type: `shell:startup`
3. Press Enter (opens Startup folder)
4. Create shortcut:
   - Right-click → New → Shortcut
   - Location: `C:\Program Files\FrameworkControl\framework-control.exe`
   - Name: Framework Control
5. Done! App will start with Windows

---

## 🗑️ Uninstall

### Method 1: Using Install Script
```powershell
# Open PowerShell as Administrator
cd "C:\Users\Taylor Allred\Documents\Files\projects\framework-control-0.4.2\framework-control-0.4.2"
.\install_system.ps1 -Uninstall
```

### Method 2: Manual Removal

1. **Stop the application:**
   - Close Framework Control if running

2. **Remove shortcuts:**
   - Delete from Start Menu
   - Delete from Desktop (if exists)

3. **Remove installation:**
   - Delete: `C:\Program Files\FrameworkControl\`

4. **Remove from PATH (if added):**
   - Settings → System → About → Advanced system settings
   - Environment Variables → System Variables → Path
   - Remove: `C:\Program Files\FrameworkControl`

---

## 📊 Installation Summary

| Item | Status |
|------|--------|
| Executable | ✅ Installed |
| Start Menu | ✅ Created |
| Desktop Shortcut | ✅ Optional |
| System PATH | ✅ Optional |
| Configuration | ✅ Created |
| Total Size | ~6-7 MB |

---

## 🎯 Quick Test

### Verify Installation:

1. **Open Start Menu:**
   - Press Windows key
   - Type: "Framework Control"
   - Should appear in search results

2. **Launch Application:**
   - Click to open
   - GUI window should appear

3. **Check Functionality:**
   - View temperatures (should update every 2 seconds)
   - View fan speeds
   - Try switching fan modes

---

## 🐛 Troubleshooting

### Issue: "Can't find Framework Control in Start Menu"
**Solution:** 
- Check: `C:\ProgramData\Microsoft\Windows\Start Menu\Programs\`
- Shortcut should be there: `Framework Control.lnk`
- Try searching for "framework-control.exe"

### Issue: "Application won't start"
**Solution:**
- Run as Administrator
- Check: `C:\Program Files\FrameworkControl\framework-control.exe` exists
- View logs in application folder

### Issue: "framework_tool not found"
**Solution:**
- Install framework_tool: `winget install FrameworkComputer.framework_tool`
- Or download from: https://github.com/FrameworkComputer/framework-system

### Issue: "Access Denied"
**Solution:**
- Run installer as Administrator
- Check antivirus isn't blocking

---

## 📝 System Requirements

- **OS:** Windows 10/11
- **RAM:** 100 MB minimum
- **Disk Space:** 10 MB
- **Permissions:** Administrator (for installation only)
- **Optional:** framework_tool for Framework laptop features
- **Optional:** RyzenAdj for AMD power management

---

## 🎊 All Done!

Framework Control is now installed and ready to use!

### Quick Access:
- 🔍 **Search:** Windows key → "Framework Control"
- 🖥️ **Desktop:** Double-click shortcut (if created)
- ⌨️ **Terminal:** Type `framework-control`

**Enjoy controlling your Framework laptop! 🚀**

---

## 📞 Support

- **Issues:** Check logs in installation folder
- **Updates:** Pull latest from GitHub
- **Community:** Framework Community Forum
- **Documentation:** README.md in installation folder

---

## ✨ Pro Tips

1. **Pin to Taskbar:** Right-click → Pin to taskbar for quick access
2. **Keyboard Shortcut:** Create custom Windows shortcut key
3. **Auto-Start:** Add to Startup folder for automatic launch
4. **Multiple Profiles:** Copy config.json to create presets
5. **Backup Config:** Save your fan curves in separate files

**Happy Framework laptop controlling! 🎉**