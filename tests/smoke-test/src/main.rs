#![deny(warnings)]

use bitflags::bitflags;

bitflags! {
    #[derive(Debug)]
    pub struct Flags: u32 {
        const A = 0b00000001;
        const B = 0b00000010;
        const C = 0b00000100;
        const ABC = Flags::A.bits() | Flags::B.bits() | Flags::C.bits();

        const _ = !0;
    }
}

fn main() {
    println!("{:?}", Flags::ABC);
}
