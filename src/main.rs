// modifications to old uni project

const MEMORY_SIZE: usize = 1024 * 64;
const START_ADDRESS: u64 = 0x1000;


#[derive(Default, Clone, Copy, Debug)]
struct PState {
    n: bool,
    z: bool,
    c: bool,
    v: bool,
}

struct Cpu {
    regs: [u64; 31],
    sp: u64,
    pc: u64,
    pstate: PState,
    memory: Box<[u8; MEMORY_SIZE]>,
    halted: bool,
}


#[derive(Debug, Clone, Copy)]
enum Instruction {
    MovImm { rd: u8, imm: u16 },
    AddImm { rd: u8, rn: u8, imm: u16 },
    SubReg { rd: u8, rn: u8, rm: u8 },
    CmpImm { rn: u8, imm: u16 },
    B { offset: i32 },
    BEq { offset: i32 },
    Ldr { rt: u8, addr: u64 },
    Str { rt: u8, addr: u64 },
    Halt,
    Undefined(u32),
}


impl Cpu {
    fn new() -> Self {
        Self {
            regs: [0; 31],
            sp: MEMORY_SIZE as u64 - 8,
            pc: START_ADDRESS,
            pstate: PState::default(),
            memory: Box::new([0; MEMORY_SIZE]),
            halted: false,
        }
    }

    fn read_reg(&self, reg: u8) -> u64 {
        if reg < 31 { self.regs[reg as usize] } else { 0 }
    }

    fn write_reg(&mut self, reg: u8, val: u64) {
        if reg < 31 { self.regs[reg as usize] = val; }
    }

    fn load_program(&mut self, program: &[u32]) {
        let start = START_ADDRESS as usize;
        for (i, &word) in program.iter().enumerate() {
            let addr = start + (i * 4);
            self.memory[addr]     = (word & 0xFF) as u8;
            self.memory[addr + 1] = ((word >> 8) & 0xFF) as u8;
            self.memory[addr + 2] = ((word >> 16) & 0xFF) as u8;
            self.memory[addr + 3] = ((word >> 24) & 0xFF) as u8;
        }
    }

    fn fetch(&mut self) -> u32 {
        let addr = self.pc as usize;
        if addr + 4 > MEMORY_SIZE {
            panic!("Bus Error: PC out of bounds at 0x{:X}", addr);
        }

        let b0 = self.memory[addr] as u32;
        let b1 = self.memory[addr + 1] as u32;
        let b2 = self.memory[addr + 2] as u32;
        let b3 = self.memory[addr + 3] as u32;

        b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
    }

    fn decode(&self, raw: u32) -> Instruction {
        let opcode = ((raw >> 24) & 0xFF) as u8;
        let reg_a  = ((raw >> 16) & 0xFF) as u8;
        let reg_b  = ((raw >> 8) & 0xFF) as u8;
        let imm    = (raw & 0xFF) as u16;
        let branch_offset = (raw & 0xFFFF) as i16 as i32;

        match opcode {
            0x01 => Instruction::MovImm { rd: reg_a, imm },
            0x02 => Instruction::AddImm { rd: reg_a, rn: reg_b, imm },
            0x03 => Instruction::SubReg { rd: reg_a, rn: reg_b, rm: imm as u8 },
            0x04 => Instruction::CmpImm { rn: reg_a, imm },
            0x05 => Instruction::B { offset: branch_offset },
            0x06 => Instruction::BEq { offset: branch_offset },
            0x07 => Instruction::Ldr { rt: reg_a, addr: imm as u64 },
            0x08 => Instruction::Str { rt: reg_a, addr: imm as u64 },
            0xFF => Instruction::Halt,
            _    => Instruction::Undefined(raw),
        }
    }

    fn execute(&mut self, inst: Instruction) {
        match inst {
            Instruction::MovImm { rd, imm } => {
                self.write_reg(rd, imm as u64);
            }
            Instruction::AddImm { rd, rn, imm } => {
                let val_n = self.read_reg(rn);
                let result = val_n.wrapping_add(imm as u64);
                self.write_reg(rd, result);
            }
            Instruction::SubReg { rd, rn, rm } => {
                let val_n = self.read_reg(rn);
                let val_m = self.read_reg(rm);
                let result = val_n.wrapping_sub(val_m);
                self.write_reg(rd, result);
            }
            Instruction::CmpImm { rn, imm } => {
                let val_n = self.read_reg(rn);
                let val_imm = imm as u64;
                let result = val_n.wrapping_sub(val_imm);

                self.pstate.z = result == 0;
                self.pstate.n = (result as i64) < 0;
            }
            Instruction::B { offset } => {
                self.pc = (self.pc as i64 + offset as i64 - 4) as u64;
            }
            Instruction::BEq { offset } => {
                if self.pstate.z {
                     self.pc = (self.pc as i64 + offset as i64 - 4) as u64;
                }
            }
            Instruction::Ldr { rt, addr } => {
                if (addr as usize) < MEMORY_SIZE {
                   let val = self.memory[addr as usize] as u64;
                   self.write_reg(rt, val);
                }
            }
            Instruction::Str { rt, addr } => {
                 if (addr as usize) < MEMORY_SIZE {
                    let val = self.read_reg(rt);
                    self.memory[addr as usize] = (val & 0xFF) as u8;
                 }
            }
            Instruction::Halt => {
                self.halted = true;
            }
            Instruction::Undefined(op) => {
                println!("Fault: Undefined Instruction 0x{:X} at 0x{:X}", op, self.pc);
                self.halted = true;
            }
        }
    }
    fn step(&mut self) {
        if self.halted {
            return;
        }
        let raw = self.fetch();
        let inst = self.decode(raw);
        println!("[0x{:04X}] {:08X} -> {:?}", self.pc, raw, inst);
        self.execute(inst);
        match inst {
            Instruction::B { .. } | Instruction::BEq { .. } => {
               self.pc += 4;
            }
            _ => {
                self.pc += 4;
            }
        }
    }
    fn dump_state(&self) {
        println!("---------------------------------------------------");
        println!("CPU State:");
        println!("PC: 0x{:04X}  SP: 0x{:04X}  FLAGS: [N:{} Z:{} C:{} V:{}]", self.pc, self.sp, self.pstate.n, self.pstate.z, self.pstate.c, self.pstate.v);

        for i in 0..4 {
            print!("X{:02}: 0x{:016X}   ", i, self.regs[i]);
            if i % 2 == 1 { println!(); }
        }
        println!("---------------------------------------------------");
    }
}


fn get_firmware() -> Vec<u32> {
    vec![
        0x01_00_00_0A,      //   MOV X0, #10
        0x01_01_00_00,      //   MOV X1, #0
        0x01_02_00_01,      //   MOV X2, #1
        0x02_01_01_02,      //   ADD X1, X1, #2
        0x03_00_00_02,      //   SUB X0, X0, X2
        0x04_00_00_00,      //   CMP X0, #0
        0x06_00_00_08,      //   B.EQ +8
        0x05_00_00_F0,      //   B -16
        0xFF_00_00_00,      //   HALT
    ]
}


fn main() {
    println!("Initializing ARMv8 Simulation...");
    let mut cpu = Cpu::new();
    let firmware = get_firmware();
    cpu.load_program(&firmware);

    while !cpu.halted {
        cpu.step();
    }
    println!("Execution Halted.");
    cpu.dump_state();

    if cpu.read_reg(1) == 20 {
        println!("SIMULATION SUCCESS: Result in X1 is 20.");
    }
}