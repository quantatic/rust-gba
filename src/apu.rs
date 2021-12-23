use crate::bus::DataAccess;

// stub for APU registers for now

#[derive(Debug, Default)]
pub struct Apu {
    sound_bias: u16,
}

impl Apu {
    pub fn read_sound_bias<DataAccessType>(&self, index: u32) -> DataAccessType
    where
        u16: DataAccess<DataAccessType>,
    {
        self.sound_bias.get_data(index)
    }

    pub fn write_sound_bias<DataAccessType>(&mut self, value: DataAccessType, index: u32)
    where
        u16: DataAccess<DataAccessType>,
    {
        self.sound_bias = self.sound_bias.set_data(value, index);
    }
}
