//! Port decoding and host/device interoperability through the public interface.

use game_toolkit_dosbus::{
    DmaChannel, DmaDirection, DmaRequest, DmaWidth, IoDevice, IrqLine, IrqRequest, PortRange, Tick,
    TickAccumulator,
};

const PORTS: [PortRange; 2] = [PortRange::new(0x220, 0x22f), PortRange::single(0x388)];

#[test]
fn port_maps_include_endpoints_and_detect_conflicts() {
    assert!(PORTS[0].contains(0x220));
    assert!(PORTS[0].contains(0x22f));
    assert!(!PORTS[0].contains(0x230));
    assert!(!PORTS[0].contains(0x21f));
    assert!(PORTS[0].overlaps(PortRange::new(0x22f, 0x330)));
    assert!(!PORTS[0].overlaps(PortRange::new(0x230, 0x330)));
    assert!(PORTS[1].overlaps(PORTS[1]));
    assert_eq!(PORTS[0].start(), 0x220);
    assert_eq!(PORTS[0].end(), 0x22f);
    let all = PortRange::new(0, u16::MAX);
    assert!(all.contains(0));
    assert!(all.contains(u16::MAX));
    assert!(all.overlaps(PortRange::single(u16::MAX)));
}

#[test]
#[should_panic(expected = "reversed I/O port range")]
fn reversed_port_ranges_are_rejected() {
    let _ = PortRange::new(0x389, 0x388);
}

#[test]
fn hardware_line_numbers_are_validated() {
    for number in 0..=u8::MAX {
        let irq = IrqLine::new(number);
        assert_eq!(irq.is_some(), number < 16);
        if let Some(irq) = irq {
            assert_eq!(irq.number(), number);
        }
        let dma = DmaChannel::new(number);
        assert_eq!(dma.is_some(), matches!(number, 0..=3 | 5..=7));
        if let Some(dma) = dma {
            assert_eq!(dma.number(), number);
            assert_eq!(
                dma.width(),
                if number < 4 {
                    DmaWidth::Byte
                } else {
                    DmaWidth::Word
                }
            );
        }
    }
}

// A minimal external adapter exercises object safety, timestamped accesses,
// autonomous activity, and level-sensitive request observation through the trait.
struct Alarm {
    now: Tick,
    due: Option<Tick>,
}

impl IoDevice for Alarm {
    fn ports(&self) -> &[PortRange] {
        &PORTS
    }

    fn advance_to(&mut self, at: Tick) {
        assert!(at >= self.now);
        self.now = at;
    }

    fn outb(&mut self, at: Tick, _port: u16, value: u8) {
        self.advance_to(at);
        self.due = Some(at + Tick(u64::from(value)));
    }

    fn inb(&mut self, at: Tick, _port: u16) -> u8 {
        self.advance_to(at);
        let asserted = self.irq_pending(at);
        self.due = None;
        u8::from(asserted)
    }

    fn irq_request(&self, at: Tick) -> Option<IrqRequest> {
        Some(IrqRequest {
            line: IrqLine::new(5).unwrap(),
            asserted: self.due.is_some_and(|due| at >= due),
        })
    }
}

#[test]
fn hosts_can_own_devices_through_the_shared_interface() {
    let mut alarm = Alarm {
        now: Tick::ZERO,
        due: None,
    };
    let device: &mut dyn IoDevice = &mut alarm;
    assert!(device.ports().iter().any(|range| range.contains(0x220)));
    device.outb(Tick(10), 0x220, 5);
    device.advance_to(Tick(14));
    assert!(!device.irq_pending(Tick(14)));
    device.advance_to(Tick(15));
    assert!(device.irq_pending(Tick(15)));
    // Polling does not acknowledge the IRQ; the device's read protocol does.
    assert!(device.irq_request(Tick(15)).unwrap().asserted);
    assert_eq!(device.inb(Tick(15), 0x220), 1);
    assert!(!device.irq_pending(Tick(15)));
    assert_eq!(device.dma_request(Tick(15)), None);
}

#[test]
fn request_types_can_be_passed_between_host_and_device_threads() {
    fn assert_send<T: Send>() {}
    assert_send::<Tick>();
    assert_send::<TickAccumulator>();
    assert_send::<IrqRequest>();
    assert_send::<DmaRequest>();
    let request = DmaRequest {
        channel: DmaChannel::new(5).unwrap(),
        direction: DmaDirection::MemoryToDevice,
    };
    assert_eq!(request.channel.width(), DmaWidth::Word);
}
