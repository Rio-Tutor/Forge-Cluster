use crate::device::Device;

#[derive(Default)]
pub struct DeviceRegistry {
    devices: Vec<Device>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, device: Device) -> Result<(), String> {
        if self.devices.iter().any(|existing| existing.id == device.id) {
            return Err(format!("Device ID {} is already registered", device.id));
        }

        self.devices.push(device);
        Ok(())
    }

    pub fn count(&self) -> usize {
        self.devices.len()
    }

    pub fn get_by_id(&self, id: u32) -> Option<&Device> {
        self.devices.iter().find(|device| device.id == id)
    }

    pub fn get_all(&self) -> &[Device] {
        &self.devices
    }
}
