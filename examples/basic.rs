use gatt::*;

#[repr(transparent)]
pub struct CharacteristicA(Characteristic);
#[repr(transparent)]
pub struct DescriptorA(Descriptor);

struct Attribute {
    /// The type of the attribute as a UUID16, EG "Primary Service" or "Anaerobic Heart Rate Lower Limit"
    pub att_type: usize,
    /// Unique server-side identifer for attribute
    pub handle: usize,
    /// Attribute values can be any fixed length or variable length octet array, which if too large
    /// can be sent across multiple PDUs
    pub value: &'static mut [u8],
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
pub struct ServiceA(Service);

#[repr(transparent)]
pub struct ServiceB(Service);

#[repr(transparent)]
pub struct AttributeA(Attribute);
#[repr(transparent)]
pub struct AttributeB(Attribute);
#[repr(transparent)]
pub struct AttributeC(Attribute);
#[repr(transparent)]
pub struct AttributeD(Attribute);

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
        attribute: AttributeD
    },
    service: ServiceB {
        attribute: AttributeD { 1 }
    },
}

fn main() {
    let mut server = gatt_server::GattServer::take().unwrap();
    server.service_a().kek();
    let mut s = server.service_a();
    let mut c = s.characteristic_a();
    let mut d = c.descriptor_a();
    let a = d.attribute_a();
    let v = a.get();

    server
        .service_a()
        .characteristic_a()
        .descriptor_a()
        .b()
        .set(v);
}
