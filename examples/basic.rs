use gatt::*;

struct CharacteristicA {}

struct DescriptorA {}

struct Attribute {
    /// The type of the attribute as a UUID16, EG "Primary Service" or "Anaerobic Heart Rate Lower Limit"
    pub att_type: usize,
    /// Unique server-side identifer for attribute
    pub handle: usize,
    /// Attribute values can be any fixed length or variable length octet array, which if too large
    /// can be sent across multiple PDUs
    pub value: &'static [u8],
}

struct Descriptor {
    attributes: &'static [Attribute],
}

struct Characteristic {
    attributes: &'static [Attribute],
    descriptors: &'static [Descriptor],
}

struct Service {
    attributes: &'static [Attribute],
    characteristics: &'static [Characteristic],
}

trait ServiceTrait {}

#[repr(transparent)]
struct ServiceA(Service);

#[repr(transparent)]
struct ServiceB(Service);

impl ServiceA {
    fn kek(&mut self) {}
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

fn main() {
    // cli! {
    //     f32
    // }
    // println!("{}", kek);

    gatt_server::services::service_a().kek();
}
