use gatt::*;
fn main() {
    // cli! {
    //     f32
    // }
    // println!("{}", kek);
}

struct ServiceA {}

struct CharacteristicA {}

struct DescriptorA {}

struct Attribute<'a> {
    /// The type of the attribute as a UUID16, EG "Primary Service" or "Anaerobic Heart Rate Lower Limit"
    pub att_type: usize,
    /// Unique server-side identifer for attribute
    pub handle: usize,
    /// Attribute values can be any fixed length or variable length octet array, which if too large
    /// can be sent across multiple PDUs
    pub value: &'a [u8],
}

gatt_server! {
    service: ServiceA {
        characteristic: CharacteristicA {
            descriptor: DescriptorA {
                attribute: AttributeA { 3 },
                attribute b: AttributeB { 3 },
                attribute c: AttributeC { 3 },
            }
        },
        characteristic: CharacteristicA {
            descriptor: DescriptorA {
                attribute: AttributeA { 5 },
                attribute b: AttributeB,
                attribute c: AttributeC { 7 },
            }
        },
        attribute: AttributeD
    },
    service: ServiceB {
        characteristic: CharacteristicA {
            descriptor: DescriptorA {
                attribute: AttributeA,
                attribute b: AttributeB,
                attribute c: AttributeC,
            }
        },
        attribute: AttributeD { 1 }
    },
}
