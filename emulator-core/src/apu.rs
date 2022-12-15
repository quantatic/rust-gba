use crate::DataAccess;

// stub for APU registers for now

#[derive(Clone, Debug, Default)]
pub struct Apu {
    sound_bias: u16,
}

impl Apu {
    pub fn read_sound_bias<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.sound_bias.get_data(index)
    }

    pub fn write_sound_bias<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.sound_bias = self.sound_bias.set_data(value, index);
    }
}
