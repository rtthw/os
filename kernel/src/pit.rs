//! # Programmable Interval Timer (PIT)


const PIT_CHANNEL_2_PORT: u16 = 0x42;
const PIT_COMMAND_REGISTER_PORT: u16 = 0x43;
const PIT_CHANNEL_2_GATE_PORT: u16 = 0x61;

const PIT_DEFAULT_FREQUENCY_HZ: u32 = 1_193_182; // 1.19 MHz


// https://wiki.osdev.org/PIT#Mode_1_%E2%80%93_Hardware_Re-triggerable_One-shot
// https://wiki.osdev.org/PC_Speaker#Through_the_Programmable_Interval_Timer_(PIT)
pub fn sleep(microseconds: u16) {
    let frequency = PIT_DEFAULT_FREQUENCY_HZ / (1_000_000 / microseconds as u32);

    unsafe {
        // Set the speaker channel 2 to be controlled by the PIT, with the following
        // config:
        //      | 10       | Select channel 2
        //      |   11     | Access mode: lobyte/hibyte
        //      |     001  | Operating mode: hardware re-triggerable one-shot
        //      |        0 | Four-digit BCD mode
        let prev = read_gate();
        write_gate(prev & 0b_11111101 | 0b_00000001);
        write_command(0b_10110010);

        // Set the frequency. We read from PS/2 port 0x60 between writing the low and
        // high bytes to act as a kind of delay/acknowledgement.
        write_channel_2(frequency as u8);
        _ = x86_port::read_u8(0x60); // ACK
        write_channel_2((frequency >> 8) as u8);

        // Reset the one-shot counter by clearing bit 0, then setting it again.
        let prev = read_gate() & 0b_11111110;
        write_gate(prev);
        write_gate(prev | 0b_00000001);

        // Finally, we can wait for the timer to finish, which happens when bit 5 is
        // cleared (the speaker moves "in").
        while read_gate() & 0b_00100000 != 0 {
            core::hint::spin_loop();
        }
    }
}

#[inline(always)]
unsafe fn read_gate() -> u8 {
    unsafe { x86_port::read_u8(PIT_CHANNEL_2_GATE_PORT) }
}

#[inline(always)]
unsafe fn write_gate(value: u8) {
    unsafe { x86_port::write_u8(PIT_CHANNEL_2_GATE_PORT, value) }
}

#[inline(always)]
unsafe fn write_channel_2(value: u8) {
    unsafe { x86_port::write_u8(PIT_CHANNEL_2_PORT, value) }
}

#[inline(always)]
unsafe fn write_command(value: u8) {
    unsafe { x86_port::write_u8(PIT_COMMAND_REGISTER_PORT, value) }
}
