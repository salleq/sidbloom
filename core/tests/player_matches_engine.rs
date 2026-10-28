//! Runs the real 6502 player in a CPU emulator and checks that every SID register write,
//! in order, frame by frame, equals what the Rust engine produces.
use mos6502::cpu::CPU;
use mos6502::instruction::Nmos6502;
use mos6502::memory::Bus;
use sidcore::engine::render_log;
use sidcore::export::{export_sid, pack_song, INIT_ADDR, LOAD_ADDR, PLAY_ADDR};
use sidcore::*;

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

fn player_log(song: &Song, frames: usize) -> (Vec<Vec<(u8, u8)>>, u64) {
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

fn assert_same(song: &Song, frames: usize, what: &str) -> u64 {
    let expected = render_log(song, frames);
    let (got, worst) = player_log(song, frames);
    for (i, (e, g)) in expected.iter().zip(&got).enumerate() {
        assert_eq!(e, g, "{what}: first mismatch at frame {i}");
    }
    worst
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 16) as u32
    }
    fn below(&mut self, n: u32) -> u32 { self.next() % n }
}

fn random_song(seed: u64) -> Song {
    let mut r = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
    let mut s = Song::new();
    s.pal = r.below(2) == 0;
    s.speed = 1 + r.below(8) as u8;
    s.filter = FilterSettings {
        cutoff: r.below(2048) as u16,
        resonance: r.below(16) as u8,
        routing: r.below(8) as u8,
        mode: [0, 0x10, 0x20, 0x40, 0x30][r.below(5) as usize],
        volume: r.below(16) as u8,
    };
    s.instruments = (0..1 + r.below(5))
        .map(|i| {
            let n = 1 + r.below(6) as usize;
            let waves = [0x11u8, 0x21, 0x41, 0x81, 0x15, 0x13, 0x40];
            Instrument {
                name: format!("i{i}"),
                attack_decay: r.below(256) as u8,
                sustain_release: r.below(256) as u8,
                pulse_width: r.below(4096) as u16,
                wave_table: (0..n)
                    .map(|_| WaveStep { wave: waves[r.below(7) as usize], semis: (r.below(41) as i32 - 20) as i8 })
                    .collect(),
                wave_loop: if r.below(2) == 0 { Some(r.below(n as u32) as u8) } else { None },
                pulse_speed: if r.below(2) == 0 { 0 } else { (r.below(256) as i32 - 128) as i8 },
                pulse_min: r.below(4096) as u16,
                pulse_max: if r.below(4) == 0 { r.below(8192) as u16 } else { 2048 + r.below(2048) as u16 },
            }
        })
        .collect();
    let n_inst = s.instruments.len() as u32;
    s.patterns = (0..1 + r.below(4))
        .map(|_| {
            let mut p = Pattern::new(1 + r.below(16) as usize);
            for c in &mut p.rows {
                if r.below(3) == 0 {
                    c.note = match r.below(10) { 0 => NOTE_OFF, _ => 1 + r.below(95) as u8 };
                    c.inst = if r.below(3) == 0 { 0 } else { 1 + r.below(n_inst + 1) as u8 };
                }
                if r.below(3) == 0 {
                    c.fx = [fx::ARPEGGIO, fx::SLIDE_UP, fx::SLIDE_DOWN, fx::VIBRATO, fx::SET_SPEED, fx::NONE, fx::FILTER_CUTOFF, fx::FILTER_RES_ROUTE, fx::FILTER_MODE_VOL, fx::FILTER_SWEEP][r.below(10) as usize];
                    c.fx_param = match c.fx {
                        fx::SET_SPEED => r.below(10) as u8,
                        fx::SLIDE_UP | fx::SLIDE_DOWN => r.below(256) as u8,
                        _ => r.below(256) as u8,
                    };
                }
            }
            p
        })
        .collect();
    let n_pat = s.patterns.len() as u32;
    for o in &mut s.orders {
        *o = (0..1 + r.below(4)).map(|_| r.below(n_pat) as u8).collect();
    }
    s
}

#[test]
fn default_song() {
    let mut s = Song::new();
    s.patterns[0].rows[0] = Cell { note: 58, inst: 1, ..Default::default() };
    s.patterns[0].rows[8] = Cell { note: 62, inst: 1, ..Default::default() };
    assert_same(&s, 400, "default");
}

#[test]
fn randomized_songs() {
    let mut worst = 0;
    for seed in 1..=300 {
        let s = random_song(seed);
        worst = worst.max(assert_same(&s, 500, &format!("seed {seed}")));
    }
    // PAL frame = 19656 cycles; stay well inside so the player is safe for real hardware
    assert!(worst < 5000, "play() worst case {worst} cycles");
    eprintln!("worst-case play() cycles: {worst}");
}

#[test]
fn psid_header_is_valid() {
    let mut s = Song::new();
    s.title = "Test Tune".into();
    s.author = "Me".into();
    let sid = export_sid(&s, false).unwrap();
    assert_eq!(&sid[0..4], b"PSID");
    assert_eq!(u16::from_be_bytes([sid[4], sid[5]]), 2);
    assert_eq!(u16::from_be_bytes([sid[6], sid[7]]), 0x7C);
    assert_eq!(u16::from_be_bytes([sid[8], sid[9]]), 0, "load address comes from the data");
    assert_eq!(u16::from_be_bytes([sid[10], sid[11]]), 0x1000);
    assert_eq!(u16::from_be_bytes([sid[12], sid[13]]), 0x1003);
    assert_eq!(&sid[0x16..0x1F], b"Test Tune");
    assert_eq!(sid[0x1F], 0);
    assert_eq!(u16::from_be_bytes([sid[0x76], sid[0x77]]), 0b01 << 2 | 0b01 << 4); // PAL, 6581
    assert_eq!(&sid[0x7C..0x7E], &[0x00, 0x10], "little-endian load address");
}
/// If 64tass is installed, player.bin must be exactly what player.asm assembles to.
#[test]
fn player_bin_is_up_to_date() {
    let out = std::env::temp_dir().join("sidbloom_player_check.bin");
    let run = std::process::Command::new("64tass")
        .args(["-b", "-o"]).arg(&out)
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/player/player.asm"))
        .output();
    let Ok(run) = run else { eprintln!("64tass not installed, skipping"); return };
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stdout));
    let built = std::fs::read(&out).unwrap();
    let committed = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/player/player.bin")).unwrap();
    assert!(built == committed, "player/player.bin is stale: run `64tass -b -o player/player.bin player/player.asm`");
}
