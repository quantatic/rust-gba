use std::ops::RangeInclusive;

pub trait BitManipulation {
    fn match_mask(self, mask: Self, result: Self) -> bool;

    fn get_bit(self, offset: usize) -> bool;

    fn set_bit(self, offset: usize, set: bool) -> Self;

    fn get_bit_range(self, bit_range: RangeInclusive<usize>) -> Self;

    fn set_bit_range(self, value: Self, bit_range: RangeInclusive<usize>) -> Self;
}

macro_rules! bit_manipulation_impl {
    ($type:ty) => {
        impl BitManipulation for $type {
            #[inline]
            fn match_mask(self, mask: Self, result: Self) -> bool {
                (self & mask) == result
            }

            #[inline]
            fn get_bit(self, offset: usize) -> bool {
                let mask = 1 << offset;
                (self & mask) == mask
            }

            #[inline]
            fn set_bit(self, offset: usize, set: bool) -> Self {
                let mask = 1 << offset;
                if set {
                    self | mask
                } else {
                    self & (!mask)
                }
            }

            #[inline]
            fn get_bit_range(self, bit_range: RangeInclusive<usize>) -> Self {
                if bit_range.is_empty() {
                    return 0;
                }

                let shift = *bit_range.start();
                let num_ones = bit_range.end() - bit_range.start() + 1;
                let mask = (2 as $type).wrapping_pow(num_ones as u32).wrapping_sub(1) << shift;
                (self & mask) >> shift
            }

            #[inline]
            fn set_bit_range(self, value: Self, bit_range: RangeInclusive<usize>) -> Self {
                if bit_range.is_empty() {
                    return self;
                }

                let shift = *bit_range.start();
                let num_ones = bit_range.end() - bit_range.start() + 1;
                let mask = (2 as $type).wrapping_pow(num_ones as u32).wrapping_sub(1) << shift;
                ((value << shift) & mask) | (self & (!mask))
            }
        }
    };
}

bit_manipulation_impl!(u8);
bit_manipulation_impl!(u16);
bit_manipulation_impl!(u32);
bit_manipulation_impl!(u64);
