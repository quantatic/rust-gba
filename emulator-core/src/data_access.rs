pub trait DataAccess<T> {
    fn set_data(self, value: T, index: u32) -> Self;

    fn get_data(self, index: u32) -> T;
}

impl<T> DataAccess<T> for T {
    fn set_data(self, value: T, index: u32) -> Self {
        assert!(index == 0);

        value
    }

    fn get_data(self, index: u32) -> Self {
        assert!(index == 0);

        self
    }
}

impl DataAccess<u8> for u16 {
    fn set_data(self, value: u8, index: u32) -> Self {
        assert!(index < 2);

        let shift = 8 * (index & 0b1);
        (self & (!(0xFF << shift))) | (u16::from(value) << shift)
    }

    fn get_data(self, index: u32) -> u8 {
        assert!(index < 2);

        let shift = 8 * (index & 0b1);
        (self >> shift) as u8
    }
}

impl DataAccess<u8> for u32 {
    fn set_data(self, value: u8, index: u32) -> Self {
        assert!(index < 4);

        let shift = 8 * (index & 0b11);
        (self & (!(0xFF << shift))) | (u32::from(value) << shift)
    }

    fn get_data(self, index: u32) -> u8 {
        assert!(index < 4);

        let shift = 8 * (index & 0b11);
        (self >> shift) as u8
    }
}

impl DataAccess<u16> for u32 {
    fn set_data(self, value: u16, index: u32) -> Self {
        assert!(index < 2);

        let shift = 16 * (index & 0b1);
        (self & (!(0xFF_FF << shift))) | (u32::from(value) << shift)
    }

    fn get_data(self, index: u32) -> u16 {
        assert!(index < 2);

        let shift = 16 * (index & 0b1);
        (self >> shift) as u16
    }
}

impl DataAccess<u32> for u64 {
    fn set_data(self, value: u32, index: u32) -> Self {
        assert!(index < 2);

        let shift = 32 * (index & 0b1);
        (self & (!(0xFF_FF_FF_FF << shift))) | (u64::from(value) << shift)
    }

    fn get_data(self, index: u32) -> u32 {
        assert!(index < 2);

        let shift = 32 * (index & 0b1);
        (self >> shift) as u32
    }
}
