//! # Programmable Interval Timer (PIT)

use x86_64::instructions::port::Port;


const PIT_CHANNEL_2_PORT: u16 = 0x42;
const PIT_COMMAND_REGISTER_PORT: u16 = 0x43;
const PIT_CHANNEL_2_GATE_PORT: u16 = 0x61;

const PIT_DEFAULT_FREQUENCY_HZ: u32 = 1_193_182; // 1.19 MHz


// https://wiki.osdev.org/PIT#Mode_1_%E2%80%93_Hardware_Re-triggerable_One-shot
// https://wiki.osdev.org/PC_Speaker#Through_the_Programmable_Interval_Timer_(PIT)
pub fn sleep(microseconds: u16) {
    let frequency = PIT_DEFAULT_FREQUENCY_HZ / (1_000_000 / microseconds as u32);

    unsafe {
        let mut pit_command = Port::<u8>::new(PIT_COMMAND_REGISTER_PORT);
        let mut pit_channel_2 = Port::<u8>::new(PIT_CHANNEL_2_PORT);
        let mut gate = Port::<u8>::new(PIT_CHANNEL_2_GATE_PORT);

        // Set the speaker channel 2 to be controlled by the PIT, with the following
        // config:
        //      | 10       | Select channel 2
        //      |   11     | Access mode: lobyte/hibyte
        //      |     001  | Operating mode: hardware re-triggerable one-shot
        //      |        0 | Four-digit BCD mode
        let prev = gate.read();
        gate.write(prev & 0b_11111101 | 0b_00000001);
        pit_command.write(0b_10110010);

        // Set the frequency. We read from PS/2 port 0x60 between writing the low and
        // high bytes to act as a kind of delay/acknowledgement.
        pit_channel_2.write(frequency as u8);
        _ = Port::<u8>::new(0x60).read(); // ACK
        pit_channel_2.write((frequency >> 8) as u8);

        // Reset the one-shot counter by clear bit 0, then setting it again.
        let prev = gate.read() & 0b_11111110;
        gate.write(prev);
        gate.write(prev | 0b_00000001);

        // Finally, we can wait for the timer to finish, which happens when bit 5 is
        // cleared (the speaker moves "in").
        while gate.read() & 0b_00100000 != 0 {
            core::hint::spin_loop();
        }
    }
}
