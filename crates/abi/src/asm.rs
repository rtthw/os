//! # x86 Assembly

pub struct Disassembler<'a> {
    pub input: &'a [u8],
    pub offset: usize,
}

impl<'a> Disassembler<'a> {
    pub const fn new(input: &'a [u8]) -> Self {
        Self { input, offset: 0 }
    }

    pub fn next(&mut self) -> Result<Instruction, DisassemblyError> {
        if self.offset == self.input.len() {
            return Err(DisassemblyError::PartialInstruction);
        }

        let opcode: u8;
        let mut rex: Option<Rex> = None;

        loop {
            let byte = self.advance()?;
            match byte {
                byte if is_rex_prefix(byte) => {
                    rex = Some(Rex(byte));
                    let next_byte = self.advance()?;
                    opcode = next_byte;

                    break;
                }
                byte => {
                    opcode = byte;
                    break;
                }
            }
        }

        match opcode {
            // ADD r/m64, r64
            0x01 => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::ADD {
                    dst: modrm.rm64_rm_operand(rex),
                    src: modrm.r64_reg_operand(rex),
                })
            }
            // OR r/m64, r64
            0x09 => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::OR {
                    dst: modrm.rm64_rm_operand(rex),
                    src: modrm.r64_reg_operand(rex),
                })
            }
            // OR r64, r/m64
            0x0B => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::OR {
                    dst: modrm.r64_reg_operand(rex),
                    src: modrm.rm64_rm_operand(rex),
                })
            }
            // 0F xx
            0x0F => self.parse_0f_instruction(rex),
            // SBB AL, imm8
            0x1C => {
                let imm = ImmediateValue::I8(i8::from_le_bytes([self.advance()?]));
                Ok(Instruction::SBB {
                    dst: Operand::Register(Register::RAX),
                    src: Operand::ImmediateValue(imm),
                })
            }
            // AND r/m64, r64
            0x21 => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::AND {
                    dst: modrm.rm64_rm_operand(rex),
                    src: modrm.r64_reg_operand(rex),
                })
            }
            // AND r64, r/m64
            0x23 => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::AND {
                    dst: modrm.r64_reg_operand(rex),
                    src: modrm.rm64_rm_operand(rex),
                })
            }
            // SUB r/m64, r64
            0x29 => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::SUB {
                    dst: modrm.rm64_rm_operand(rex),
                    src: modrm.r64_reg_operand(rex),
                })
            }
            // XOR r/m64, r64
            0x31 => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::XOR {
                    dst: modrm.rm64_rm_operand(rex),
                    src: modrm.r64_reg_operand(rex),
                })
            }
            // CMP r/m64, r64
            0x39 => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::CMP {
                    src_1: modrm.rm64_rm_operand(rex),
                    src_2: modrm.r64_reg_operand(rex),
                })
            }
            // 4X opcodes are not encodable in 64-bit mode.
            0x40..=0x4F => Err(DisassemblyError::InvalidByte),
            // PUSH r64
            0x50..=0x57 => Ok(Instruction::PUSH {
                operand: Operand::Register(
                    parse_register(opcode & 7, rex.is_some_and(|rex| rex.b())).unwrap(),
                ),
            }),
            // POP r64
            0x58..=0x5F => Ok(Instruction::POP {
                operand: Operand::Register(
                    parse_register(opcode & 7, rex.is_some_and(|rex| rex.b())).unwrap(),
                ),
            }),
            // PUSHA/PUSHAD
            0x60 => Err(DisassemblyError::InvalidByte),
            // POPA/POPAD
            0x61 => Err(DisassemblyError::InvalidByte),
            // PUSH imm32
            0x68 => {
                let imm = ImmediateValue::I32(i32::from_le_bytes([
                    self.advance()?,
                    self.advance()?,
                    self.advance()?,
                    self.advance()?,
                ]));

                Ok(Instruction::PUSH {
                    operand: Operand::ImmediateValue(imm),
                })
            }
            // PUSH imm8
            0x6A => {
                let imm = ImmediateValue::I8(i8::from_le_bytes([self.advance()?]));
                Ok(Instruction::PUSH {
                    operand: Operand::ImmediateValue(imm),
                })
            }
            // Jcc rel8
            0x70..=0x7F => {
                let cc = ConditionCode::new(opcode & 0x0F).ok_or(DisassemblyError::InvalidByte)?;
                Ok(Instruction::JCC {
                    cc,
                    offset: ImmediateValue::I8(i8::from_le_bytes([self.advance()?])),
                })
            }
            // Group 1
            0x83 => {
                let modrm = ModRm::new(self.advance()?);
                match modrm.reg {
                    // ADC r/m64, imm8
                    0x2 => {
                        let imm = ImmediateValue::I8(i8::from_le_bytes([self.advance()?]));
                        Ok(Instruction::ADC {
                            dst: modrm.rm64_rm_operand(rex),
                            src: Operand::ImmediateValue(imm),
                        })
                    }

                    _ => Err(DisassemblyError::InvalidByte),
                }
            }
            // TEST r/m64, r64
            0x85 => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::TEST {
                    src_1: modrm.rm64_rm_operand(rex),
                    src_2: modrm.r64_reg_operand(rex),
                })
            }
            // MOV r/m64, r64
            0x89 => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::MOV {
                    dst: modrm.rm64_rm_operand(rex),
                    src: modrm.r64_reg_operand(rex),
                })
            }
            // MOV r64, r/m64
            0x8B => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::MOV {
                    dst: modrm.r64_reg_operand(rex),
                    src: modrm.rm64_rm_operand(rex),
                })
            }
            // LEA r64, m
            0x8D => {
                todo!()
            }
            // CBW/CWDE/CDQE
            0x98 => {
                todo!("CBW/CWDE/CDQE")
            }
            // MOV r64, imm
            0xB8..=0xBF => {
                let dst = Operand::Register(
                    parse_register(opcode & 7, rex.is_some_and(|rex| rex.b())).unwrap(),
                );
                let src = Operand::ImmediateValue(if rex.is_some() {
                    ImmediateValue::I64(i64::from_le_bytes([
                        self.advance()?,
                        self.advance()?,
                        self.advance()?,
                        self.advance()?,
                        self.advance()?,
                        self.advance()?,
                        self.advance()?,
                        self.advance()?,
                    ]))
                } else {
                    ImmediateValue::I32(i32::from_le_bytes([
                        self.advance()?,
                        self.advance()?,
                        self.advance()?,
                        self.advance()?,
                    ]))
                });

                Ok(Instruction::MOV { src, dst })
            }
            // Group 2
            0xC0 => {
                let modrm = ModRm::new(self.advance()?);
                match modrm.reg {
                    // SHL r/m8, imm8
                    0x4 => {
                        let imm = ImmediateValue::I8(i8::from_le_bytes([self.advance()?]));
                        Ok(Instruction::SHL {
                            dst: modrm.rm64_rm_operand(rex),
                            src: Operand::ImmediateValue(imm),
                        })
                    }
                    // SHR r/m8, imm8
                    0x5 => {
                        let imm = ImmediateValue::I8(i8::from_le_bytes([self.advance()?]));
                        Ok(Instruction::SHR {
                            dst: modrm.rm64_rm_operand(rex),
                            src: Operand::ImmediateValue(imm),
                        })
                    }
                    // SAR r/m8, imm8
                    0x7 => {
                        let imm = ImmediateValue::I8(i8::from_le_bytes([self.advance()?]));
                        Ok(Instruction::SAR {
                            dst: modrm.rm64_rm_operand(rex),
                            src: Operand::ImmediateValue(imm),
                        })
                    }

                    _ => Err(DisassemblyError::InvalidByte),
                }
            }
            // Group 2
            0xC1 => {
                let modrm = ModRm::new(self.advance()?);
                match modrm.reg {
                    // SHL r/m64, imm8
                    0x4 => {
                        let imm = ImmediateValue::I8(i8::from_le_bytes([self.advance()?]));
                        Ok(Instruction::SHL {
                            dst: modrm.rm64_rm_operand(rex),
                            src: Operand::ImmediateValue(imm),
                        })
                    }
                    // SHR r/m64, imm8
                    0x5 => {
                        let imm = ImmediateValue::I8(i8::from_le_bytes([self.advance()?]));
                        Ok(Instruction::SHR {
                            dst: modrm.rm64_rm_operand(rex),
                            src: Operand::ImmediateValue(imm),
                        })
                    }
                    // SAR r/m64, imm8
                    0x7 => {
                        let imm = ImmediateValue::I8(i8::from_le_bytes([self.advance()?]));
                        Ok(Instruction::SAR {
                            dst: modrm.rm64_rm_operand(rex),
                            src: Operand::ImmediateValue(imm),
                        })
                    }

                    _ => Err(DisassemblyError::InvalidByte),
                }
            }
            // RET
            0xC2 | 0xCA => {
                let _ = self.advance()?;
                let _ = self.advance()?;
                Ok(Instruction::RET)
            }
            // RET
            0xC3 | 0xCB => Ok(Instruction::RET),
            // INT3
            0xCC => Ok(Instruction::INT3),
            // INT imm8
            0xCD => Ok(Instruction::INT {
                vector: ImmediateValue::I8(i8::from_le_bytes([self.advance()?])),
            }),
            // INTO (Invalid in 64-bit mode)
            0xCE => Err(DisassemblyError::InvalidByte),
            // IN AL/AX/EAX, imm8
            0xE4..=0xE5 => {
                let imm = ImmediateValue::I8(i8::from_le_bytes([self.advance()?]));
                Ok(Instruction::IN {
                    dst: Register::RAX,
                    port: Operand::ImmediateValue(imm),
                })
            }
            // OUT imm8, AL/AX/EAX
            0xE6..=0xE7 => {
                let imm = ImmediateValue::I8(i8::from_le_bytes([self.advance()?]));
                Ok(Instruction::IN {
                    dst: Register::RAX,
                    port: Operand::ImmediateValue(imm),
                })
            }
            // JMP rel32
            0xE9 => {
                let imm = ImmediateValue::I32(i32::from_le_bytes([
                    self.advance()?,
                    self.advance()?,
                    self.advance()?,
                    self.advance()?,
                ]));
                Ok(Instruction::JMP {
                    operand: Operand::ImmediateValue(imm),
                })
            }
            0xEA => Err(DisassemblyError::InvalidByte),
            // JMP rel8
            0xEB => {
                let imm = ImmediateValue::I8(i8::from_le_bytes([self.advance()?]));
                Ok(Instruction::JMP {
                    operand: Operand::ImmediateValue(imm),
                })
            }
            // IN AL/AX/EAX, DX
            0xEC..=0xED => Ok(Instruction::IN {
                dst: Register::RAX,
                port: Operand::Register(Register::RDX),
            }),
            // OUT DX, AL/AX/EAX
            0xEE..=0xEF => Ok(Instruction::OUT {
                src: Register::RAX,
                port: Operand::Register(Register::RDX),
            }),
            // INT1
            0xF1 => Ok(Instruction::INT1),
            // HLT
            0xF4 => Ok(Instruction::HLT),
            // CMC
            0xF5 => Ok(Instruction::CMC),
            // Group 3
            0xF6 => {
                let modrm = ModRm::new(self.advance()?);
                match modrm.reg {
                    // NOT r/m8
                    0x2 => todo!("NOT r/m8 (F6 /2)"),
                    // NEG r/m8
                    0x3 => todo!("NEG r/m8 (F6 /3)"),
                    // IMUL r/m8
                    0x5 => todo!("IMUL r/m8 (F6 /5)"),
                    // IDIV r/m8
                    0x7 => todo!("IDIV r/m8 (F6 /7)"),

                    _ => Err(DisassemblyError::InvalidByte),
                }
            }
            // Group 3
            0xF7 => {
                let modrm = ModRm::new(self.advance()?);
                match modrm.reg {
                    // NOT r/m64
                    0x2 => todo!("NOT r/m64 (F7 /2)"),
                    // NEG r/m64
                    0x3 => todo!("NEG r/m64 (F7 /3)"),
                    // IMUL r/m64
                    0x5 => todo!("IMUL r/m64 (F7 /5)"),
                    // IDIV r/m64
                    0x7 => todo!("IDIV r/m64 (F7 /7)"),

                    _ => Err(DisassemblyError::InvalidByte),
                }
            }
            // CLC
            0xF8 => Ok(Instruction::CLC),
            // STC
            0xF9 => Ok(Instruction::STC),
            // CLI
            0xFA => Ok(Instruction::CLI),
            // STI
            0xFB => Ok(Instruction::STI),
            // CLD
            0xFC => Ok(Instruction::CLD),
            // STD
            0xFD => Ok(Instruction::STD),
            // Group 4
            0xFE => {
                todo!("INC/DEC")
            }
            // Group 5
            // FIXME: This should differentiate between JMP near and JMP far (FF /4 and FF /5).
            0xFF => {
                let modrm = ModRm::new(self.advance()?);
                match modrm.reg {
                    // INC r/m64 @ ModRM:r/m (r, w)
                    0x0 => Ok(Instruction::INC {
                        operand: modrm.rm64_rm_operand(rex),
                    }),
                    // CALL m16:64 @ ModRM:r/m (r)
                    0x2 => Ok(Instruction::CALL {
                        operand: modrm.rm64_rm_operand(rex),
                    }),
                    // JMP r/m64 @ ModRM:r/m (r)
                    0x4 => Ok(Instruction::JMP {
                        operand: modrm.rm64_rm_operand(rex),
                    }),
                    // JMP m16:64 @ ModRM:r/m (r)
                    0x5 => Ok(Instruction::JMP {
                        operand: modrm.rm64_rm_operand(rex),
                    }),

                    _ => Err(DisassemblyError::InvalidByte),
                }
            }

            other => Err(DisassemblyError::UnsupportedOpcode { opcode: other }),
        }
    }

    fn advance(&mut self) -> Result<u8, DisassemblyError> {
        if self.offset >= self.input.len() {
            return Err(DisassemblyError::PartialInstruction);
        }

        let byte = self.input[self.offset];
        self.offset += 1;

        Ok(byte)
    }

    fn parse_0f_instruction(&mut self, rex: Option<Rex>) -> Result<Instruction, DisassemblyError> {
        let opcode = self.advance()?;
        match opcode {
            0x01 => {
                let next_byte = self.advance()?;
                match next_byte {
                    0xEE => Ok(Instruction::RDPKRU),
                    0xF8 => Ok(Instruction::SWAPGS),
                    0xF9 => Ok(Instruction::RDTSCP),

                    other => {
                        let modrm = ModRm::new(other);
                        if modrm.is_memory() {
                            todo!()
                        } else {
                            Err(DisassemblyError::InvalidByte)
                        }
                    }
                }
            }
            // SYSCALL
            0x05 => Ok(Instruction::SYSCALL),
            // SYSRET
            0x07 => Ok(Instruction::SYSRET),
            // RDTSC
            0x31 => Ok(Instruction::RDTSC),
            // RDMSR
            0x32 => Ok(Instruction::RDMSR),
            // RDPMC
            0x33 => Ok(Instruction::RDPMC),
            // SYSENTER
            0x34 => Ok(Instruction::SYSENTER),
            // SYSENTER
            0x35 => Ok(Instruction::SYSEXIT),
            // CMOVcc r64, r/m64
            0x40..=0x4F => {
                let cc = ConditionCode::new(opcode & 0x0F).ok_or(DisassemblyError::InvalidByte)?;
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::CMOVCC {
                    cc,
                    dst: modrm.r64_reg_operand(rex),
                    src: modrm.rm64_rm_operand(rex),
                })
            }
            // SETcc r/m8
            0x90..=0x9F => {
                let cc = ConditionCode::new(opcode & 0x0F).ok_or(DisassemblyError::InvalidByte)?;
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::SETCC {
                    cc,
                    byte: modrm.rm64_rm_operand(rex),
                })
            }
            // CPUID
            0xA2 => Ok(Instruction::CPUID),

            other => Err(DisassemblyError::UnsupportedOpcode { opcode: other }),
        }
    }
}

#[derive(Debug)]
pub enum DisassemblyError {
    InvalidByte,
    PartialInstruction,
    UnsupportedOpcode { opcode: u8 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Instruction {
    ADC {
        dst: Operand,
        src: Operand,
    },
    ADD {
        dst: Operand,
        src: Operand,
    },
    AND {
        dst: Operand,
        src: Operand,
    },
    CALL {
        operand: Operand,
    },
    CLC,
    CLD,
    CLI,
    CMC,
    CMOVCC {
        cc: ConditionCode,
        dst: Operand,
        src: Operand,
    },
    CMP {
        src_1: Operand,
        src_2: Operand,
    },
    CPUID,
    HLT,
    IN {
        dst: Register,
        port: Operand,
    },
    INC {
        operand: Operand,
    },
    INT {
        vector: ImmediateValue,
    },
    INVLPG {
        src: Operand,
    },
    INT1,
    INT3,
    INTO,
    JCC {
        cc: ConditionCode,
        offset: ImmediateValue,
    },
    JMP {
        operand: Operand,
    },
    LEA {
        dst: Operand,
        src: Operand,
    },
    MOV {
        dst: Operand,
        src: Operand,
    },
    MUL {
        operand: Operand,
    },
    OR {
        dst: Operand,
        src: Operand,
    },
    OUT {
        src: Register,
        port: Operand,
    },
    PUSH {
        operand: Operand,
    },
    POP {
        operand: Operand,
    },
    RDMSR,
    RDPKRU,
    RDPMC,
    RDTSC,
    RDTSCP,
    RET,
    SAR {
        dst: Operand,
        src: Operand,
    },
    SBB {
        dst: Operand,
        src: Operand,
    },
    SETCC {
        cc: ConditionCode,
        byte: Operand,
    },
    SHL {
        dst: Operand,
        src: Operand,
    },
    SHR {
        dst: Operand,
        src: Operand,
    },
    STC,
    STD,
    STI,
    SUB {
        dst: Operand,
        src: Operand,
    },
    SWAPGS,
    SYSCALL,
    SYSENTER,
    SYSEXIT,
    SYSRET,
    TEST {
        src_1: Operand,
        src_2: Operand,
    },
    XOR {
        dst: Operand,
        src: Operand,
    },
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Operand {
    Register(Register),
    ImmediateValue(ImmediateValue),
    Memory { base: Option<Register>, disp: i32 },
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Register {
    RAX,
    RCX,
    RDX,
    RBX,
    RSP,
    RBP,
    RSI,
    RDI,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,

    RIP,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ImmediateValue {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
}

/// The REX prefix of an instruction.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct Rex(pub u8);

impl Rex {
    /// REX.W
    #[inline]
    pub const fn w(&self) -> bool {
        (self.0 & 0b1000) != 0
    }

    /// REX.R
    #[inline]
    pub const fn r(&self) -> bool {
        (self.0 & 0b0100) != 0
    }

    /// REX.X
    #[inline]
    pub const fn x(&self) -> bool {
        (self.0 & 0b0010) != 0
    }

    /// REX.B
    #[inline]
    pub const fn b(&self) -> bool {
        (self.0 & 0b0001) != 0
    }
}

/// The ModR/M byte of an instruction.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ModRm {
    /// Addressing mode (the `mod` field).
    pub md: u8,
    /// Register or opcode extension (the `reg` field).
    pub reg: u8,
    /// Register or memory addressing (the `rm` field).
    pub rm: u8,
}

impl ModRm {
    pub const fn new(byte: u8) -> Self {
        Self {
            md: (byte >> 6) & 0b11,
            reg: (byte >> 3) & 0b111,
            rm: byte & 0b111,
        }
    }

    pub const fn is_memory(&self) -> bool {
        self.md != 3
    }

    pub const fn has_sib(&self) -> bool {
        self.is_memory() && self.rm == 4
    }

    pub const fn is_rip_relative(&self) -> bool {
        self.md == 0 && self.rm == 5
    }

    /// Parse the `reg` field as an `r64` operand.
    pub const fn r64_reg_operand(&self, rex: Option<Rex>) -> Operand {
        let rex_b = match rex {
            Some(rex) => rex.b(),
            None => false,
        };
        Operand::Register(
            parse_register(self.reg, rex_b).expect("ModR/M:reg should be less than 8"),
        )
    }

    /// Parse the `rm` field as an `r/m64` operand.
    pub const fn rm64_rm_operand(&self, rex: Option<Rex>) -> Operand {
        let rex_b = match rex {
            Some(rex) => rex.b(),
            None => false,
        };
        if self.is_memory() {
            Operand::Memory {
                base: if self.has_sib() {
                    None
                } else if self.is_rip_relative() {
                    Some(Register::RIP)
                } else {
                    parse_register(self.rm, rex_b)
                },
                disp: 0,
            }
        } else {
            Operand::Register(
                parse_register(self.rm, rex_b).expect("ModR/M:rm should be less than 8"),
            )
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum ConditionCode {
    /// Overflow (OF = 1).
    O = 0x0,
    /// No overflow (OF = 0).
    NO = 0x1,
    /// Below, Carry (CF = 1).
    B = 0x2,
    /// Not below, No carry (CF = 0).
    NB = 0x3,
    /// Equal, Zero (ZF = 1).
    Z = 0x4,
    /// Not equal, Not zero (ZF = 0).
    NZ = 0x5,
    /// Below or equal (CF = 1 or ZF = 1).
    BE = 0x6,
    /// Above (CF = 0 and ZF = 0).
    A = 0x7,
    /// Sign (SF = 1).
    S = 0x8,
    /// No sign (SF = 0).
    NS = 0x9,
    /// Parity (PF = 1).
    P = 0xA,
    /// No parity (PF = 0).
    NP = 0xB,
    /// Less (SF != OF).
    L = 0xC,
    /// Greater or equal (SF = OF).
    GE = 0xD,
    /// Less or equal (ZF = 1 or SF != OF).
    LE = 0xE,
    /// Greater (ZF = 0 and SF = OF).
    G = 0xF,
}

impl ConditionCode {
    pub const fn new(byte: u8) -> Option<Self> {
        match byte {
            0x0 => Some(Self::O),
            0x1 => Some(Self::NO),
            0x2 => Some(Self::B),
            0x3 => Some(Self::NB),
            0x4 => Some(Self::Z),
            0x5 => Some(Self::NZ),
            0x6 => Some(Self::BE),
            0x7 => Some(Self::A),
            0x8 => Some(Self::S),
            0x9 => Some(Self::NS),
            0xA => Some(Self::P),
            0xB => Some(Self::NP),
            0xC => Some(Self::L),
            0xD => Some(Self::GE),
            0xE => Some(Self::LE),
            0xF => Some(Self::G),
            _ => None,
        }
    }
}

#[inline]
const fn is_rex_prefix(byte: u8) -> bool {
    byte & 0xF0 == 0x40
}

const fn parse_register(index: u8, rex: bool) -> Option<Register> {
    match (index, rex) {
        (0, false) => Some(Register::RAX),
        (1, false) => Some(Register::RCX),
        (2, false) => Some(Register::RDX),
        (3, false) => Some(Register::RBX),
        (4, false) => Some(Register::RSP),
        (5, false) => Some(Register::RBP),
        (6, false) => Some(Register::RSI),
        (7, false) => Some(Register::RDI),
        (0, true) => Some(Register::R8),
        (1, true) => Some(Register::R9),
        (2, true) => Some(Register::R10),
        (3, true) => Some(Register::R11),
        (4, true) => Some(Register::R12),
        (5, true) => Some(Register::R13),
        (6, true) => Some(Register::R14),
        (7, true) => Some(Register::R15),
        _ => None,
    }
}



#[cfg(test)]
mod tests {
    use super::{Operand::*, Register::*, *};

    extern crate std;

    #[test]
    fn smoke() {
        #[rustfmt::skip]
        let instructions = [
            0x55,
            0x48, 0x89, 0xe5,
            0x48, 0x8b, 0x06,
            0x48, 0x89, 0x07,
            0x5d,
            0xc3,
        ];
        let expected = [
            Instruction::PUSH {
                operand: Register(RBP),
            },
            Instruction::MOV {
                dst: Register(RBP),
                src: Register(RSP),
            },
            Instruction::MOV {
                dst: Register(RAX),
                src: Memory {
                    base: Some(RSI),
                    disp: 0,
                },
            },
            Instruction::MOV {
                dst: Memory {
                    base: Some(RDI),
                    disp: 0,
                },
                src: Register(RAX),
            },
            Instruction::POP {
                operand: Register(RBP),
            },
            Instruction::RET,
        ];

        let mut disassembler = Disassembler::new(&instructions);
        let mut i = 0;
        while let Ok(instruction) = disassembler.next() {
            // std::println!("{instruction:?}");
            assert_eq!(instruction, expected[i]);
            i += 1;
        }
        assert_eq!(disassembler.offset, instructions.len());
    }

    #[test]
    fn print() {
        #[rustfmt::skip]
        let instructions = [
            0x48, 0x39, 0xd6,
            0x77, 0x12,
            0x48, 0xbf, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x41, 0xff, 0xd0,
        ];

        let mut disassembler = Disassembler::new(&instructions);
        while let Ok(instruction) = disassembler.next() {
            std::println!("{instruction:?}");
        }
    }
}
