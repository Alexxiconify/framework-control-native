
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RyzenAdjInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tdp_watts: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thermal_limit_c: Option<u32>,
}

/// Parse output of `ryzenadj --info --dump-table`
/// Strategy: scan table rows and extract key limits as watts (rounded)
pub fn parse_info(text: &str) -> RyzenAdjInfo {
    let mut info = RyzenAdjInfo::default();
    if text.is_empty() {
        return info;
    }
    let mut limits_w: Vec<f32> = Vec::new();

    // Parse lines like: | STAPM LIMIT         |    67.000 | stapm-limit        |
    for line in text.lines() {
        let l = line.trim();
        if !l.starts_with('|') || l.starts_with("|-") {
            continue;
        }

        // Split by '|' and extract parts
        let parts: Vec<&str> = l.split('|').collect();
        if parts.len() < 3 {
            continue;
        }

        let name = parts[1].trim().to_ascii_uppercase();
        let value_str = parts[2].trim();

        if let Ok(v) = value_str.parse::<f32>() {
            // Collect power limit candidates
            if name.contains("STAPM LIMIT")
                || name.contains("PPT LIMIT FAST")
                || name.contains("PPT LIMIT SLOW")
            {
                limits_w.push(v);
            }
            // Thermal limit
            if name.contains("THM LIMIT CORE") || name.contains("TCTL") {
                info.thermal_limit_c = Some(v.round() as u32);
            }
        }
    }

    if !limits_w.is_empty() {
        let min_w = limits_w.into_iter().fold(f32::INFINITY, f32::min);
        if min_w.is_finite() {
            info.tdp_watts = Some(min_w.round().max(1.0) as u32);
        }
    }
    info
}
