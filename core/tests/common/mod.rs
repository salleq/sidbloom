//! Shared 6502 harness: runs the real player in an emulator and captures SID register writes.
#![allow(dead_code)]
use mos6502::cpu::CPU;
use mos6502::instruction::Nmos6502;
use mos6502::memory::Bus;
use sidcore::engine::render_log;
use sidcore::export::{pack_song, INIT_ADDR, LOAD_ADDR, PLAY_ADDR};
use sidcore::Song;

struct Ram {
    mem: Vec<u8>,
    sid_writes: Vec<(u8, u8)>,
}

impl Bus for Ram {
    fn get_byte(&mut self, a: u16) -> u8 { self.mem[a as usize] }
    fn set_byte(&mut self, a: u16, v: u8) {
        if (0xD400..=0xD418).contains(&a) { self.sid_writes.push(((a - 0xD400) as u8, v)); }
        self.mem[a as usize] = v;
    }
}

const SENTINEL: u16 = 0x0300;

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

