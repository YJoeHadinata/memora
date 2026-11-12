use std::sync::Arc;

use crate::models::ThermalSnapshot;
use crate::system::thermal::ThermalCollector;

pub struct ThermalService {
    collector: Arc<dyn ThermalCollector>,
}

impl ThermalService {
    pub fn new(collector: Arc<dyn ThermalCollector>) -> Self {
        Self { collector }
    }

    pub fn snapshot(&self) -> Result<ThermalSnapshot, String> {
        self.collector.collect_thermal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ThermalPressureState;

    struct DummyCollector;
    impl ThermalCollector for DummyCollector {
        fn collect_thermal(&self) -> Result<ThermalSnapshot, String> {
            Ok(ThermalSnapshot {
                soc_temp_avg: Some(55.0),
                soc_temp_max: Some(60.0),
                storage_temp: Some(45.0),
                thermal_state: ThermalPressureState::Nominal,
                sensors: vec![],
            })
        }
    }

    #[test]
    fn test_thermal_service_snapshot() {
        let service = ThermalService::new(Arc::new(DummyCollector));
        let snap = service.snapshot().expect("snapshot ok");
        assert_eq!(snap.soc_temp_avg, Some(55.0));
        assert_eq!(snap.thermal_state, ThermalPressureState::Nominal);
    }
}
