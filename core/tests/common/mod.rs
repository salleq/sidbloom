//! Shared 6502 harness: runs the real player in an emulator and captures SID register writes.
#![allow(dead_code)]
use mos6502::cpu::CPU;
use mos6502::instruction::Nmos6502;
use mos6502::memory::Bus;
use sidcore::engine::render_log;
use sidcore::export::{pack_song, INIT_ADDR, LOAD_ADDR, PLAY_ADDR};
use sidcore::Song;

pub struct Ram {
    pub mem: Vec<u8>,
    pub sid_writes: Vec<(u8, u8)>,
}

impl Bus for Ram {
    fn get_byte(&mut self, a: u16) -> u8 { self.mem[a as usize] }
    fn set_byte(&mut self, a: u16, v: u8) {
        if (0xD400..=0xD418).contains(&a) { self.sid_writes.push(((a - 0xD400) as u8, v)); }
        self.mem[a as usize] = v;
    }
}

pub const SENTINEL: u16 = 0x0300;

fn call(cpu: &mut CPU<Ram, Nmos6502>, addr: u16) {
    let ret = SENTINEL - 1;
    cpu.memory.mem[0x1FF] = (ret >> 8) as u8;
    cpu.memory.mem[0x1FE] = ret as u8;
    cpu.registers.stack_pointer.0 = 0xFD;
    cpu.registers.program_counter = addr;
    let start = cpu.cycles;
    while cpu.registers.program_counter != SENTINEL {
        cpu.single_step();
        assert!(cpu.cycles - start < 200_000, "player ran away (pc={:04x})", cpu.registers.program_counter);
    }
}

pub fn player_log(song: &Song, frames: usize) -> (Vec<Vec<(u8, u8)>>, u64) {
    let image = pack_song(song).unwrap();
    let mut mem = vec![0u8; 65536];
    mem[LOAD_ADDR as usize..LOAD_ADDR as usize + image.len()].copy_from_slice(&image);
    let mut cpu = CPU::new(Ram { mem, sid_writes: vec![] }, Nmos6502);
    call(&mut cpu, INIT_ADDR);
    cpu.memory.sid_writes.clear();
    let mut log = vec![];
    let mut worst = 0;
    for _ in 0..frames {
        let c0 = cpu.cycles;
        call(&mut cpu, PLAY_ADDR);
        worst = worst.max(cpu.cycles - c0);
        log.push(std::mem::take(&mut cpu.memory.sid_writes));
    }
    (log, worst)
}

pub fn assert_same(song: &Song, frames: usize, what: &str) -> u64 {
    let expected = render_log(song, frames);
    let (got, worst) = player_log(song, frames);
    for (i, (e, g)) in expected.iter().zip(&got).enumerate() {
        assert_eq!(e, g, "{what}: first mismatch at frame {i}");
    }
    worst
}


/// Boots a `.prg` the way a C64 would after `RUN`: executes the BASIC `SYS` target, then fires the
/// IRQ vector once per frame. Returns the SID writes per frame. KERNAL's IRQ tail is a stub `RTS` at $EA31.
pub fn prg_log(prg: &[u8], frames: usize) -> Vec<Vec<(u8, u8)>> {
    let load = u16::from_le_bytes([prg[0], prg[1]]) as usize;
    assert_eq!(load, 0x0801, "BASIC programs load at $0801");
    let mut mem = vec![0u8; 65536];
    mem[load..load + prg.len() - 2].copy_from_slice(&prg[2..]);
    // BASIC line: link, line number, SYS token, ascii address, 0
    assert_eq!(mem[0x0805], 0x9E, "SYS token");
    let sys: String = mem[0x0806..].iter().take_while(|&&b| b != 0).map(|&b| b as char).collect();
    let sys: u16 = sys.parse().expect("SYS address");
    mem[0xEA31] = 0x60; // RTS: stands in for the KERNAL IRQ tail
    let mut cpu = CPU::new(Ram { mem, sid_writes: vec![] }, Nmos6502);
    // run SEI ... CLI until it reaches the idle `JMP *` loop
    cpu.registers.program_counter = sys;
    cpu.registers.stack_pointer.0 = 0xFD;
    let mut idle = 0;
    for _ in 0..10_000 {
        let pc = cpu.registers.program_counter;
        cpu.single_step();
        if cpu.registers.program_counter == pc {
            idle += 1;
            if idle > 2 { break; }
        }
    }
    assert!(idle > 2, "start-up code never reached its idle loop");
    let timer = cpu.memory.mem[0xDC04] as u16 | (cpu.memory.mem[0xDC05] as u16) << 8;
    assert!(timer == 63 * 312 - 1 || timer == 65 * 263 - 1, "CIA timer = {timer}");
    assert_eq!(cpu.memory.mem[0xDC0D], 0x81, "timer interrupt enabled");
    cpu.memory.sid_writes.clear(); // init may clear the SID
    let vec = cpu.memory.mem[0x0314] as u16 | (cpu.memory.mem[0x0315] as u16) << 8;
    (0..frames)
        .map(|_| {
            // an IRQ pushes a return address; here we just call the vector and wait for the KERNAL tail
            let ret = SENTINEL - 1;
            cpu.memory.mem[0x1FF] = (ret >> 8) as u8;
            cpu.memory.mem[0x1FE] = ret as u8;
            cpu.registers.stack_pointer.0 = 0xFD;
            cpu.registers.program_counter = vec;
            let start = cpu.cycles;
            while cpu.registers.program_counter != 0xEA31 {
                cpu.single_step();
                assert!(cpu.cycles - start < 200_000, "irq ran away");
            }
            std::mem::take(&mut cpu.memory.sid_writes)
        })
        .collect()
}
