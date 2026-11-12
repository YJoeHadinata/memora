use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThermalPressureState {
    #[default]
    Nominal,
    Fair,
    Serious,
    Critical,
    Unknown,
}

impl ThermalPressureState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Nominal => "Nominal",
            Self::Fair => "Fair",
            Self::Serious => "Serious",
            Self::Critical => "Critical",
            Self::Unknown => "Unknown",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Nominal => "Cool & Optimal — no thermal throttling",
            Self::Fair => "Warm — moderate load, fans may spin up",
            Self::Serious => "Hot — thermal throttling active to protect hardware",
            Self::Critical => "Critical — severe thermal throttling, cooldown required",
            Self::Unknown => "Thermal state unavailable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorReading {
    pub name: String,
    pub label: String,
    pub temp_celsius: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ThermalSnapshot {
    pub soc_temp_avg: Option<f32>,
    pub soc_temp_max: Option<f32>,
    pub storage_temp: Option<f32>,
    pub thermal_state: ThermalPressureState,
    pub sensors: Vec<SensorReading>,
}

impl ThermalSnapshot {
    pub fn primary_temp(&self) -> Option<f32> {
        self.soc_temp_max.or(self.soc_temp_avg)
    }

    pub fn temp_summary(&self) -> String {
        match (self.soc_temp_avg, self.soc_temp_max) {
            (Some(avg), Some(max)) => format!("{:.1}°C (Peak {:.1}°C)", avg, max),
            (Some(avg), None) => format!("{:.1}°C", avg),
            (None, Some(max)) => format!("{:.1}°C", max),
            (None, None) => "N/A".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thermal_snapshot_summary() {
        let snap = ThermalSnapshot {
            soc_temp_avg: Some(62.5),
            soc_temp_max: Some(68.0),
            storage_temp: Some(48.0),
            thermal_state: ThermalPressureState::Nominal,
            sensors: vec![],
        };
        assert_eq!(snap.primary_temp(), Some(68.0));
        assert!(snap.temp_summary().contains("62.5°C"));
        assert!(snap.temp_summary().contains("68.0°C"));
    }
}
