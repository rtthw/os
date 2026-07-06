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
        let mut lock_prefix = false;
        let mut f2_prefix = false;
        let mut f3_prefix = false;
        let mut rex: Option<Rex> = None;

        loop {
            let byte = self.advance()?;
            match byte {
                0xF0 => {
                    lock_prefix = true;
                }
                0xF2 => {
                    f2_prefix = true;
                }
                0xF3 => {
                    f3_prefix = true;
                }
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

        if lock_prefix {
            todo!("Handle LOCK prefix");
        }

        match opcode {
            // ADD r/m64, r64
            0x01 => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::ADD {
                    dst: self.parse_rm64_operand(modrm, rex, true)?,
                    src: modrm.r64_reg_operand(rex, true),
                })
            }
            // OR r/m64, r64
            0x09 => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::OR {
                    dst: self.parse_rm64_operand(modrm, rex, true)?,
                    src: modrm.r64_reg_operand(rex, true),
                })
            }
            // OR r64, r/m64
            0x0B => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::OR {
                    dst: modrm.r64_reg_operand(rex, true),
                    src: self.parse_rm64_operand(modrm, rex, true)?,
                })
            }
            // 0F xx
            0x0F => self.parse_0f_instruction(rex, f2_prefix, f3_prefix),
            // SBB AL, imm8
            0x1C => {
                let imm = ImmediateValue::I8(self.advance_i8()?);
                Ok(Instruction::SBB {
                    dst: Operand::Register(Register::RAX),
                    src: Operand::ImmediateValue(imm),
                })
            }
            // AND r/m64, r64
            0x21 => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::AND {
                    dst: self.parse_rm64_operand(modrm, rex, true)?,
                    src: modrm.r64_reg_operand(rex, true),
                })
            }
            // AND r64, r/m64
            0x23 => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::AND {
                    dst: modrm.r64_reg_operand(rex, true),
                    src: self.parse_rm64_operand(modrm, rex, true)?,
                })
            }
            // SUB r/m64, r64
            0x29 => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::SUB {
                    dst: self.parse_rm64_operand(modrm, rex, true)?,
                    src: modrm.r64_reg_operand(rex, true),
                })
            }
            // XOR r/m64, r64
            0x31 => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::XOR {
                    dst: self.parse_rm64_operand(modrm, rex, true)?,
                    src: modrm.r64_reg_operand(rex, true),
                })
            }
            // CMP r/m64, r64
            0x39 => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::CMP {
                    src_1: self.parse_rm64_operand(modrm, rex, true)?,
                    src_2: modrm.r64_reg_operand(rex, true),
                })
            }
            // 4X opcodes are not encodable in 64-bit mode.
            0x40..=0x4F => Err(DisassemblyError::InvalidByte),
            // PUSH r64
            0x50..=0x57 => Ok(Instruction::PUSH {
                operand: Operand::Register(
                    parse_register(opcode & 7, rex.is_some_and(|rex| rex.r())).unwrap(),
                ),
            }),
            // POP r64
            0x58..=0x5F => Ok(Instruction::POP {
                operand: Operand::Register(
                    parse_register(opcode & 7, rex.is_some_and(|rex| rex.r())).unwrap(),
                ),
            }),
            // PUSHA/PUSHAD
            0x60 => Err(DisassemblyError::InvalidByte),
            // POPA/POPAD
            0x61 => Err(DisassemblyError::InvalidByte),
            // PUSH imm32
            0x68 => Ok(Instruction::PUSH {
                operand: Operand::ImmediateValue(ImmediateValue::I32(self.advance_i32()?)),
            }),
            // PUSH imm8
            0x6A => Ok(Instruction::PUSH {
                operand: Operand::ImmediateValue(ImmediateValue::I8(self.advance_i8()?)),
            }),
            // Jcc rel8
            0x70..=0x7F => {
                let cc = ConditionCode::new(opcode & 0x0F).ok_or(DisassemblyError::InvalidByte)?;
                Ok(Instruction::JCC {
                    cc,
                    offset: ImmediateValue::I8(self.advance_i8()?),
                })
            }
            // Immediate Group 1
            0x80..=0x83 => {
                if opcode == 0x82 {
                    // Not encodable in 64-bit mode.
                    return Err(DisassemblyError::InvalidByte);
                }
                let modrm = ModRm::new(self.advance()?);
                let dst = self.parse_rm64_operand(modrm, rex, true)?;
                let src = Operand::ImmediateValue(match opcode {
                    0x80 | 0x83 => ImmediateValue::I8(self.advance_i8()?),
                    _ => ImmediateValue::I32(self.advance_i32()?),
                });
                match modrm.reg {
                    // ADD
                    0x0 => Ok(Instruction::ADD { dst, src }),
                    // OR
                    0x1 => Ok(Instruction::OR { dst, src }),
                    // ADC
                    0x2 => Ok(Instruction::ADC { dst, src }),
                    // SBB
                    0x3 => Ok(Instruction::SBB { dst, src }),
                    // AND
                    0x4 => Ok(Instruction::AND { dst, src }),
                    // SUB
                    0x5 => Ok(Instruction::SUB { dst, src }),
                    // XOR
                    0x6 => Ok(Instruction::XOR { dst, src }),
                    // CMP
                    0x7 => Ok(Instruction::CMP {
                        src_1: dst,
                        src_2: src,
                    }),

                    // This shouldn't be reachable.
                    _ => Err(DisassemblyError::InvalidByte),
                }
            }
            // TEST r/m8, r8
            0x84 => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::TEST {
                    src_1: self.parse_rm64_operand(modrm, rex, true)?,
                    src_2: modrm.r64_reg_operand(rex, true),
                })
            }
            // TEST r/m64, r64
            0x85 => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::TEST {
                    src_1: self.parse_rm64_operand(modrm, rex, true)?,
                    src_2: modrm.r64_reg_operand(rex, true),
                })
            }
            // XCHG r/m8, r8
            0x86 => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::XCHG {
                    dst: self.parse_rm64_operand(modrm, rex, true)?,
                    src: modrm.r64_reg_operand(rex, true),
                })
            }
            // XCHG r/m64, r64
            0x87 => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::XCHG {
                    dst: self.parse_rm64_operand(modrm, rex, true)?,
                    src: modrm.r64_reg_operand(rex, true),
                })
            }
            // MOV r/m8, r8
            0x88 => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::MOV {
                    dst: self.parse_rm64_operand(modrm, rex, true)?,
                    src: modrm.r64_reg_operand(rex, true),
                })
            }
            // MOV r/m64, r64
            0x89 => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::MOV {
                    dst: self.parse_rm64_operand(modrm, rex, true)?,
                    src: modrm.r64_reg_operand(rex, true),
                })
            }
            // MOV r8, r/m8
            0x8A => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::MOV {
                    dst: modrm.r64_reg_operand(rex, true),
                    src: self.parse_rm64_operand(modrm, rex, true)?,
                })
            }
            // MOV r64, r/m64
            0x8B => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::MOV {
                    dst: modrm.r64_reg_operand(rex, true),
                    src: self.parse_rm64_operand(modrm, rex, true)?,
                })
            }
            // LEA r64, m
            0x8D => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::LEA {
                    dst: modrm.r64_reg_operand(rex, true),
                    src: self.parse_rm64_operand(modrm, rex, false)?,
                })
            }
            // PAUSE
            0x90 if f3_prefix => Ok(Instruction::PAUSE),
            // NOP (alias for `XCHG rax, rax`)
            0x90 => Ok(Instruction::NOP),
            // XCHG rax, r64
            0x91..=0x97 => Ok(Instruction::XCHG {
                dst: Operand::Register(Register::RAX),
                src: Operand::Register(
                    parse_register(opcode & 7, rex.is_some_and(|rex| rex.r())).unwrap(),
                ),
            }),
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
                    ImmediateValue::I64(self.advance_i64()?)
                } else {
                    ImmediateValue::I32(self.advance_i32()?)
                });

                Ok(Instruction::MOV { src, dst })
            }
            // Group 2
            0xC0 => {
                let modrm = ModRm::new(self.advance()?);
                match modrm.reg {
                    // SHL r/m8, imm8
                    0x4 => Ok(Instruction::SHL {
                        dst: self.parse_rm64_operand(modrm, rex, true)?,
                        src: Operand::ImmediateValue(ImmediateValue::I8(self.advance_i8()?)),
                    }),
                    // SHR r/m8, imm8
                    0x5 => Ok(Instruction::SHR {
                        dst: self.parse_rm64_operand(modrm, rex, true)?,
                        src: Operand::ImmediateValue(ImmediateValue::I8(self.advance_i8()?)),
                    }),
                    // SAR r/m8, imm8
                    0x7 => Ok(Instruction::SAR {
                        dst: self.parse_rm64_operand(modrm, rex, true)?,
                        src: Operand::ImmediateValue(ImmediateValue::I8(self.advance_i8()?)),
                    }),

                    _ => Err(DisassemblyError::InvalidByte),
                }
            }
            // Group 2
            0xC1 => {
                let modrm = ModRm::new(self.advance()?);
                match modrm.reg {
                    // SHL r/m64, imm8
                    0x4 => Ok(Instruction::SHL {
                        dst: self.parse_rm64_operand(modrm, rex, true)?,
                        src: Operand::ImmediateValue(ImmediateValue::I8(self.advance_i8()?)),
                    }),
                    // SHR r/m64, imm8
                    0x5 => Ok(Instruction::SHR {
                        dst: self.parse_rm64_operand(modrm, rex, true)?,
                        src: Operand::ImmediateValue(ImmediateValue::I8(self.advance_i8()?)),
                    }),
                    // SAR r/m64, imm8
                    0x7 => Ok(Instruction::SAR {
                        dst: self.parse_rm64_operand(modrm, rex, true)?,
                        src: Operand::ImmediateValue(ImmediateValue::I8(self.advance_i8()?)),
                    }),

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
                vector: ImmediateValue::I8(self.advance_i8()?),
            }),
            // INTO (Invalid in 64-bit mode)
            0xCE => Err(DisassemblyError::InvalidByte),
            // IN AL/AX/EAX, imm8
            0xE4..=0xE5 => Ok(Instruction::IN {
                dst: Register::RAX,
                port: Operand::ImmediateValue(ImmediateValue::I8(self.advance_i8()?)),
            }),
            // OUT imm8, AL/AX/EAX
            0xE6..=0xE7 => Ok(Instruction::IN {
                dst: Register::RAX,
                port: Operand::ImmediateValue(ImmediateValue::I8(self.advance_i8()?)),
            }),
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
            0xEB => Ok(Instruction::JMP {
                operand: Operand::ImmediateValue(ImmediateValue::I8(self.advance_i8()?)),
            }),
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
                        operand: self.parse_rm64_operand(modrm, rex, true)?,
                    }),
                    // CALL m16:64 @ ModRM:r/m (r)
                    0x2 => Ok(Instruction::CALL {
                        operand: self.parse_rm64_operand(modrm, rex, false)?,
                    }),
                    // JMP r/m64 @ ModRM:r/m (r)
                    0x4 => Ok(Instruction::JMP {
                        operand: self.parse_rm64_operand(modrm, rex, false)?,
                    }),
                    // JMP m16:64 @ ModRM:r/m (r)
                    0x5 => Ok(Instruction::JMP {
                        operand: self.parse_rm64_operand(modrm, rex, false)?,
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

    fn advance_i8(&mut self) -> Result<i8, DisassemblyError> {
        Ok(i8::from_le_bytes([self.advance()?]))
    }

    fn advance_i32(&mut self) -> Result<i32, DisassemblyError> {
        Ok(i32::from_le_bytes([
            self.advance()?,
            self.advance()?,
            self.advance()?,
            self.advance()?,
        ]))
    }

    fn advance_i64(&mut self) -> Result<i64, DisassemblyError> {
        Ok(i64::from_le_bytes([
            self.advance()?,
            self.advance()?,
            self.advance()?,
            self.advance()?,
            self.advance()?,
            self.advance()?,
            self.advance()?,
            self.advance()?,
        ]))
    }

    fn parse_0f_instruction(
        &mut self,
        rex: Option<Rex>,
        f2_prefix: bool,
        f3_prefix: bool,
    ) -> Result<Instruction, DisassemblyError> {
        let opcode = self.advance()?;
        match opcode {
            0x01 => {
                let next_byte = self.advance()?;
                match next_byte {
                    0xC6 if f2_prefix => Ok(Instruction::RDMSRLIST),
                    0xC8 => Ok(Instruction::MONITOR),
                    0xC9 => Ok(Instruction::MWAIT),
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
                    dst: modrm.r64_reg_operand(rex, false),
                    src: self.parse_rm64_operand(modrm, rex, false)?,
                })
            }
            // SETcc r/m8
            0x90..=0x9F => {
                let cc = ConditionCode::new(opcode & 0x0F).ok_or(DisassemblyError::InvalidByte)?;
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::SETCC {
                    cc,
                    byte: self.parse_rm64_operand(modrm, rex, false)?,
                })
            }
            // CPUID
            0xA2 => Ok(Instruction::CPUID),
            // TZCNT r64, r/m64
            0xBC if f3_prefix => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::TZCNT {
                    dst: modrm.r64_reg_operand(rex, false),
                    src: self.parse_rm64_operand(modrm, rex, false)?,
                })
            }
            // BSF r64, r/m64
            0xBC => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::BSF {
                    dst: modrm.r64_reg_operand(rex, false),
                    src: self.parse_rm64_operand(modrm, rex, false)?,
                })
            }
            // LZCNT r64, r/m64
            0xBD if f3_prefix => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::LZCNT {
                    dst: modrm.r64_reg_operand(rex, false),
                    src: self.parse_rm64_operand(modrm, rex, false)?,
                })
            }
            // BSR r64, r/m64
            0xBD => {
                let modrm = ModRm::new(self.advance()?);
                Ok(Instruction::BSR {
                    dst: modrm.r64_reg_operand(rex, false),
                    src: self.parse_rm64_operand(modrm, rex, false)?,
                })
            }

            0xC7 => {
                let modrm = ModRm::new(self.advance()?);
                match modrm.reg {
                    // RDRAND r64
                    0x6 => Ok(Instruction::RDRAND {
                        dst: modrm.rm_register(rex.is_some_and(|rex| rex.b())),
                    }),
                    // RDPID r64
                    0x7 if f3_prefix => Ok(Instruction::RDPID {
                        dst: modrm.rm_register(false),
                    }),
                    // RDSEED r64
                    0x7 => Ok(Instruction::RDSEED {
                        dst: modrm.rm_register(rex.is_some_and(|rex| rex.b())),
                    }),

                    _ => Err(DisassemblyError::InvalidByte),
                }
            }

            other => Err(DisassemblyError::UnsupportedOpcode { opcode: other }),
        }
    }

    /// Parse the `rm` field of the ModR/M byte as an `r/m64` operand.
    fn parse_rm64_operand(
        &mut self,
        modrm: ModRm,
        rex: Option<Rex>,
        use_rex_r: bool,
    ) -> Result<Operand, DisassemblyError> {
        let rex_b = rex.is_some_and(|rex| rex.b());
        let rex_x = rex.is_some_and(|rex| rex.x());
        let rex = match rex {
            Some(rex) => {
                if use_rex_r {
                    rex.r()
                } else {
                    rex.b()
                }
            }
            None => false,
        };

        Ok(if modrm.is_memory() {
            if modrm.has_sib() {
                let sib = Sib::new(self.advance()?);
                let disp = match sib.disp_size(&modrm) {
                    1 => self.advance_i8()? as i32,
                    4 => self.advance_i32()?,
                    _ => 0,
                };

                Operand::Memory {
                    base: sib.base_register(modrm.md, rex_b),
                    index: sib.index_register(rex_x),
                    scale: sib.scale_factor(),
                    disp,
                }
            } else {
                let disp = match modrm.disp_size() {
                    1 => self.advance_i8()? as i32,
                    4 => self.advance_i32()?,
                    _ => 0,
                };

                Operand::Memory {
                    base: if modrm.is_rip_relative() {
                        Some(Register::RIP)
                    } else {
                        parse_register(modrm.rm, rex)
                    },
                    index: None,
                    scale: 1,
                    disp,
                }
            }
        } else {
            Operand::Register(modrm.rm_register(rex))
        })
    }
}

#[derive(Debug, PartialEq)]
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
    BSF {
        dst: Operand,
        src: Operand,
    },
    BSR {
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
    LZCNT {
        dst: Operand,
        src: Operand,
    },
    MONITOR,
    MOV {
        dst: Operand,
        src: Operand,
    },
    MUL {
        operand: Operand,
    },
    MWAIT,
    NEG {
        operand: Operand,
    },
    NOP,
    OR {
        dst: Operand,
        src: Operand,
    },
    OUT {
        src: Register,
        port: Operand,
    },
    PAUSE,
    PUSH {
        operand: Operand,
    },
    POP {
        operand: Operand,
    },
    RDMSR,
    RDMSRLIST,
    RDPID {
        dst: Register,
    },
    RDPKRU,
    RDPMC,
    RDTSC,
    RDTSCP,
    RDRAND {
        dst: Register,
    },
    RDSEED {
        dst: Register,
    },
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
    TZCNT {
        dst: Operand,
        src: Operand,
    },
    XCHG {
        dst: Operand,
        src: Operand,
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
    Memory {
        base: Option<Register>,
        index: Option<Register>,
        scale: u8,
        disp: i32,
    },
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

    pub const fn disp_size(&self) -> u8 {
        match self.md {
            0 if self.rm == 5 => 4,
            1 => 1,
            2 => 4,
            _ => 0,
        }
    }

    pub const fn rm_register(&self, rex: bool) -> Register {
        parse_register(self.rm, rex).expect("ModR/M:rm should be less than 8")
    }

    /// Parse the `reg` field as an `r64` operand.
    pub const fn r64_reg_operand(&self, rex: Option<Rex>, use_rex_r: bool) -> Operand {
        let rex = match rex {
            Some(rex) => {
                if use_rex_r {
                    rex.r()
                } else {
                    rex.b()
                }
            }
            None => false,
        };
        Operand::Register(parse_register(self.reg, rex).expect("ModR/M:reg should be less than 8"))
    }
}

/// The SIB byte of an instruction.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Sib {
    /// Scale factor (1, 2, 4, 8).
    pub scale: u8,
    /// Index register.
    pub index: u8,
    /// Base register.
    pub base: u8,
}

impl Sib {
    pub const fn new(byte: u8) -> Self {
        Self {
            scale: (byte >> 6) & 0b11,
            index: (byte >> 3) & 0b111,
            base: byte & 0b111,
        }
    }

    pub const fn scale_factor(&self) -> u8 {
        match self.scale {
            1 => 2,
            2 => 4,
            3 => 8,
            _ => 1,
        }
    }

    pub const fn disp_size(&self, modrm: &ModRm) -> u8 {
        match modrm.md {
            0 if self.base == 5 => 4,
            1 => 1,
            2 => 4,
            _ => 0,
        }
    }

    pub const fn index_register(&self, rex_x: bool) -> Option<Register> {
        if self.index == 4 {
            None
        } else {
            parse_register(self.index, rex_x)
        }
    }

    pub const fn base_register(&self, md: u8, rex_b: bool) -> Option<Register> {
        if self.base == 5 && md == 0 {
            None
        } else {
            parse_register(self.base, rex_b)
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
                    index: None,
                    scale: 1,
                    disp: 0,
                },
            },
            Instruction::MOV {
                dst: Memory {
                    base: Some(RDI),
                    index: None,
                    scale: 1,
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
        assert_eq!(i, expected.len());
    }

    #[test]
    fn parse_with_sib_and_disp() {
        #[rustfmt::skip]
        let instructions = [
            0x4a, 0x8b, 0x04, 0x17,                         // mov  (%rdi,%r10,1),%rax
            0x4e, 0x8b, 0x5c, 0x17, 0x08,                   // mov  0x8(%rdi,%r10,1),%r11
            0x4e, 0x8b, 0x04, 0x12,                         // mov  (%rdx,%r10,1),%r8
            0x4a, 0x8b, 0x5c, 0x12, 0x08,                   // mov  0x8(%rdx,%r10,1),%rbx
            0x4a, 0x89, 0x84, 0xed, 0x28, 0xfd, 0xff, 0xff, // mov  %rax,-0x2d8(%rbp,%r13,8)
            0x8b, 0x3c, 0x81,                               // mov  (%rcx,%rax,4),%edi
            0x89, 0x8c, 0x85, 0x18, 0xff, 0xff, 0xff,       // mov  %ecx,-0xe8(%rbp,%rax,4)
            0x48, 0x8b, 0x75, 0xc8,                         // mov  -0x38(%rbp),%rsi
            0x4c, 0x8d, 0x61, 0xff,                         // lea  -0x1(%rcx),%r12
            0x4d, 0x8d, 0x6c, 0x24, 0xff,                   // lea  -0x1(%r12),%r13
            0x4c, 0x8d, 0x2c, 0xd5, 0xd0, 0xfd, 0xff, 0xff, // lea  -0x230(,%rdx,8),%r13
            0x8d, 0x91, 0x00, 0x00, 0x80, 0x3f,             // lea  0x3f800000(%rcx),%edx
        ];
        let expected = [
            Instruction::MOV {
                dst: Register(RAX),
                src: Memory {
                    base: Some(RDI),
                    index: Some(R10),
                    scale: 1,
                    disp: 0,
                },
            },
            Instruction::MOV {
                dst: Register(R11),
                src: Memory {
                    base: Some(RDI),
                    index: Some(R10),
                    scale: 1,
                    disp: 8,
                },
            },
            Instruction::MOV {
                dst: Register(R8),
                src: Memory {
                    base: Some(RDX),
                    index: Some(R10),
                    scale: 1,
                    disp: 0,
                },
            },
            Instruction::MOV {
                dst: Register(RBX),
                src: Memory {
                    base: Some(RDX),
                    index: Some(R10),
                    scale: 1,
                    disp: 8,
                },
            },
            Instruction::MOV {
                dst: Memory {
                    base: Some(RBP),
                    index: Some(R13),
                    scale: 8,
                    disp: -0x2d8,
                },
                src: Register(RAX),
            },
            Instruction::MOV {
                dst: Register(RDI),
                src: Memory {
                    base: Some(RCX),
                    index: Some(RAX),
                    scale: 4,
                    disp: 0,
                },
            },
            Instruction::MOV {
                dst: Memory {
                    base: Some(RBP),
                    index: Some(RAX),
                    scale: 4,
                    disp: -0xe8,
                },
                src: Register(RCX),
            },
            Instruction::MOV {
                dst: Register(RSI),
                src: Memory {
                    base: Some(RBP),
                    index: None,
                    scale: 1,
                    disp: -0x38,
                },
            },
            Instruction::LEA {
                dst: Register(R12),
                src: Memory {
                    base: Some(RCX),
                    index: None,
                    scale: 1,
                    disp: -0x1,
                },
            },
            Instruction::LEA {
                dst: Register(R13),
                src: Memory {
                    base: Some(R12),
                    index: None,
                    scale: 1,
                    disp: -0x1,
                },
            },
            Instruction::LEA {
                dst: Register(R13),
                src: Memory {
                    base: None,
                    index: Some(RDX),
                    scale: 8,
                    disp: -0x230,
                },
            },
            Instruction::LEA {
                dst: Register(RDX),
                src: Memory {
                    base: Some(RCX),
                    index: None,
                    scale: 1,
                    disp: 0x3f800000,
                },
            },
        ];

        let mut disassembler = Disassembler::new(&instructions);
        let mut i = 0;
        let mut next = disassembler.next();
        while i < expected.len() {
            // std::println!("{next:?}");
            assert_eq!(next.unwrap(), expected[i]);
            i += 1;
            next = disassembler.next();
        }
        assert_eq!(next, Err(DisassemblyError::PartialInstruction));
        assert_eq!(disassembler.offset, instructions.len());
        assert_eq!(i, expected.len());
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
