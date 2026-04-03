use nvml::{error::NvmlError, Device, Nvml};
use serde::{Deserialize, Serialize};
//use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NvidiaSensor {
    Index { index: u32 },
    BusId { busid: String },
    Uuid { uuid: String },
}

impl NvidiaSensor {
    pub fn device<'nv>(&self, nvml: &'nv Nvml) -> Result<Device<'nv>, NvmlError> {
        match self {
            NvidiaSensor::Index { index } => nvml.device_by_index(*index),
            NvidiaSensor::BusId { busid } => nvml.device_by_pci_bus_id(busid.clone()),
            NvidiaSensor::Uuid { uuid } => nvml.device_by_uuid(uuid.clone()),
        }
    }
}
