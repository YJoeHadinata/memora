mod ffi;

use std::ffi::{c_char, c_void, CStr};

use crate::models::{SensorReading, ThermalPressureState, ThermalSnapshot};
use crate::system::thermal::ThermalCollector;

use self::ffi::*;

#[derive(Default, Clone)]
pub struct MacosThermalCollector;

impl MacosThermalCollector {
    pub fn new() -> Self {
        Self
    }

    fn read_thermal_state() -> ThermalPressureState {
        unsafe {
            let cls = objc_getClass(c"NSProcessInfo".as_ptr());
            if cls.is_null() {
                return ThermalPressureState::Unknown;
            }
            let sel_pi = sel_registerName(c"processInfo".as_ptr());
            let sel_state = sel_registerName(c"thermalState".as_ptr());
            let f0: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let pi = f0(cls, sel_pi);
            if pi.is_null() {
                return ThermalPressureState::Unknown;
            }
            let f_state: extern "C" fn(*mut c_void, *mut c_void) -> isize =
                std::mem::transmute(objc_msgSend as *const ());
            match f_state(pi, sel_state) {
                0 => ThermalPressureState::Nominal,
                1 => ThermalPressureState::Fair,
                2 => ThermalPressureState::Serious,
                3 => ThermalPressureState::Critical,
                _ => ThermalPressureState::Unknown,
            }
        }
    }

    fn read_sensors() -> Vec<SensorReading> {
        unsafe {
            let client = IOHIDEventSystemClientCreate(std::ptr::null());
            if client.is_null() {
                return Vec::new();
            }

            let (dict, [page_num, usage_num, k_page, k_usage, k_prod]) =
                create_thermal_matching_dict();
            IOHIDEventSystemClientSetMatching(client, dict);
            let services = IOHIDEventSystemClientCopyServices(client);
            let mut readings = Vec::new();

            if !services.is_null() {
                let count = CFArrayGetCount(services);
                for i in 0..count {
                    let svc = CFArrayGetValueAtIndex(services, i) as *mut c_void;
                    let name_ref = IOHIDServiceClientCopyProperty(svc, k_prod);
                    let mut name = String::new();
                    if !name_ref.is_null() {
                        let mut buf = [0u8; 128];
                        if CFStringGetCString(
                            name_ref,
                            buf.as_mut_ptr() as *mut c_char,
                            128,
                            K_CF_STRING_ENCODING_UTF8,
                        ) {
                            if let Ok(s) = CStr::from_ptr(buf.as_ptr() as *const c_char).to_str() {
                                name = s.to_string();
                            }
                        }
                        CFRelease(name_ref);
                    }

                    let event =
                        IOHIDServiceClientCopyEvent(svc, K_IO_HID_EVENT_TYPE_TEMPERATURE, 0, 0);
                    if !event.is_null() {
                        let temp =
                            IOHIDEventGetFloatValue(event, IOHID_EVENT_FIELD_TEMPERATURE) as f32;
                        CFRelease(event);
                        if (1.0..130.0).contains(&temp) && !name.is_empty() {
                            let label = Self::categorize_sensor(&name);
                            readings.push(SensorReading {
                                name,
                                label,
                                temp_celsius: temp,
                            });
                        }
                    }
                }
                CFRelease(services);
            }

            for ptr in [dict, k_page, k_usage, k_prod, page_num, usage_num, client] {
                if !ptr.is_null() {
                    CFRelease(ptr);
                }
            }
            readings
        }
    }

    fn categorize_sensor(name: &str) -> String {
        let n = name.to_lowercase();
        if n.contains("tdie") {
            "CPU/GPU Core".to_string()
        } else if n.contains("nand") {
            "SSD Flash".to_string()
        } else if n.contains("tdev") {
            "Board / Device".to_string()
        } else {
            "Thermal Sensor".to_string()
        }
    }
}

impl ThermalCollector for MacosThermalCollector {
    fn collect_thermal(&self) -> Result<ThermalSnapshot, String> {
        let thermal_state = Self::read_thermal_state();
        let sensors = Self::read_sensors();

        let mut die_temps = Vec::new();
        let mut storage_temp = None;

        for s in &sensors {
            let n = s.name.to_lowercase();
            if n.contains("tdie") {
                die_temps.push(s.temp_celsius);
            } else if n.contains("nand") && storage_temp.is_none() {
                storage_temp = Some(s.temp_celsius);
            }
        }

        let (soc_temp_avg, soc_temp_max) = if !die_temps.is_empty() {
            let sum: f32 = die_temps.iter().sum();
            let avg = (sum / die_temps.len() as f32 * 10.0).round() / 10.0;
            let raw_max = die_temps.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let max = (raw_max * 10.0).round() / 10.0;
            (Some(avg), Some(max))
        } else {
            (None, None)
        };

        Ok(ThermalSnapshot {
            soc_temp_avg,
            soc_temp_max,
            storage_temp,
            thermal_state,
            sensors,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macos_thermal_collection() {
        let collector = MacosThermalCollector::new();
        let snap = collector.collect_thermal().expect("thermal collection ok");
        assert_ne!(snap.thermal_state, ThermalPressureState::Unknown);
        println!("Thermal state: {:?}", snap.thermal_state);
        println!("Sensors found: {}", snap.sensors.len());
    }
}
