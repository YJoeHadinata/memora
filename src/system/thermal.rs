use crate::models::ThermalSnapshot;

pub trait ThermalCollector: Send + Sync {
    fn collect_thermal(&self) -> Result<ThermalSnapshot, String>;
}
