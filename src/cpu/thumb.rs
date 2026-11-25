//! Thumb state.
//!
//! `--apcs=/interwork` means one module can mix ARM and Thumb encodings, and the official
//! samples do: `helloworld.mod` keeps its entry code and its `IModule` implementation in ARM
//! state but emits Thumb from `0x458` onwards — including the `AEEHANDLER` that receives every
//! event. So this file is not optional for the applet lifecycle.
//!
//! Getting a decoder wrong is not a loud failure: it misexecutes silently at some later PC and
//! costs days to localise. So Thumb is implemented against evidence, not memory. Two sources
//! are on disk:
//!
//! * a sweep of all 65536 16-bit opcodes was produced once and kept outside the repository
//!   (12 MB, not something to carry here). Every field layout below was read
//!   off that table, never recalled — the first draft of this file had two groups mislabelled
//!   and an `imm5` in the wrong nibble, which the sweep caught.
//! * real RVCT halfwords from `helloworld.mod`'s handler: `0xb5f0`, `0xb08b`, `0x4913`,
//!   `0xad05`, `0xf7ff:0xede4`. The tests execute those bytes rather than hand-written ones.
//!
//! The sweep settles which slots are *not* ARMv5TE and must stay faults: `0xB1xx`/`0xB3xx`/
//! `0xB6xx`-`0xBFxx` carry `cbz`/`rev`/`bkpt`/`it`. `0xB2xx` was on that list too, until a reference
//! pack executed `uxth` on its start-up path: the four ARMv6 extends are answered now (shared with
//! the ARM decoder), and only the byte-reverse/`cbz`/hint groups remain faults.
//!
//! It does not settle `0011x`, and the sweep's own rendering is *why*: it prints `0x300c`
//! as `adds r0, 0xc`, with the destination as the only operand, which is the ARMv6.1 reading.
//! The ARMv4T reading of the same halfword is `ADD Rd,PC,#imm8`, and the two cannot both be
//! true. [`Cpu::thumb_add_sub_immediate`] settles it with bytes from `helloworld.mod`, which is
//! the only authority that matters here.
//!
//! The shape of the state, by `bits[15:11]`:
//!
//! | bits[15:11] | group                                                          |
//! |-------------|----------------------------------------------------------------|
//! | 00000-00010 | `LSLS`/`LSRS`/`ASRS Rd,Rs,#imm5` — `MOVS` when the amount is 0  |
//! | 00011       | `ADDS`/`SUBS`, register offset and #imm3                        |
//! | 00100/00101 | `MOVS`/`CMPS Rd,#imm8`                                          |
//! | 00110/00111 | `ADDS`/`SUBS Rdn,#imm8` (ARMv4T reads these as `±PC`/`±SP`)        |
//! | 01000       | ALU by `bits[10:6]`; 16-31 hi-register `ADD`/`CMP`/`MOV`, `BX`, `BLX` |
//! | 01001       | `LDR Rd,[PC,#imm8*4]`                                          |
//! | 01010/01011 | register-offset word/half/byte and signed transfers             |
//! | 01100-01111 | immediate-offset `LDR`/`STR`, word (`bit12=0`) and byte         |
//! | 10000/10001 | immediate-offset `STRH`/`LDRH`                                 |
//! | 10010/10011 | `STR`/`LDR Rd,[SP,#imm8*4]`                                    |
//! | 10100/10101 | `ADD Rd,PC/SP,#imm8*4`                                          |
//! | 10110       | `ADD`/`SUB SP,#imm7*4` (`0xB0xx`), `PUSH` (`0xB4xx`/`0xB5xx`); the rest is ARMv6+ |
//! | 10111       | `POP` (`0xBCxx`/`0xBDxx`); `0xB8xx`-`0xBFxx` is ARMv6+/Thumb-2  |
//! | 11000/11001 | `STMIA`/`LDMIA`                                                 |
//! | 11010/11011 | `B<cond>`; `0xDExx` is ARMv6 `UDF` and `0xDFxx` `SVC` — both refused          |
//! | 11100       | `B #imm11`                                                      |
//! | 11110       | `BL`/`BLX` as a halfword pair                                   |
//! | 11101/11111 | a pair's second half, or Thumb-2 — fault                       |
//!
//! Two layout traps cost real time and are worth stating outright, because both decode to a
//! *plausible* instruction rather than to garbage. First, `Rd` normally sits in `bits[2:0]` — but
//! in `LDR` literal, the SP-relative transfers, and `ADD Rd,PC/SP` it sits in `bits[10:8]`, above
//! the eight-bit immediate. Second, the format 4 ALU group is destructive: `ands r6, r2` means
//! `r6 = r6 & r2`, so `Rd` is also the first operand and `bits[8:6]` belongs to the opcode.

use super::arm::condition_met;
use super::{shift_imm, shift_reg, Cpu, Shift, Stop};

const fn bit(n: u32) -> u32 {
    1 << n
}

/// Sign-extend the low `width` bits of `value`. The branch fields are never wider than 11 bits,
/// so `i64` keeps the shifts below lossless.
fn sign_extend(value: u32, width: u32) -> i64 {
    let mark = 1i64 << (width - 1);
    ((value as i64) ^ mark) - mark
}

impl Cpu {
    pub(crate) fn step_thumb(&mut self, pc: u32, insn: u32) -> Result<(), Stop> {
        match insn >> 11 {
            0b00000..=0b00010 => self.thumb_shift_by_immediate(insn),
            0b00011 => self.thumb_add_subtract(insn),
            0b00100 | 0b00101 => self.thumb_move_or_compare_immediate(insn),
            0b00110 | 0b00111 => self.thumb_add_sub_immediate(insn),
            0b01000 => self.thumb_alu_or_branch(insn),
            0b01001 => self.thumb_load_literal(pc, insn),
            0b01010 | 0b01011 => self.thumb_register_offset_transfer(pc, insn),
            0b01100..=0b10001 => self.thumb_immediate_offset_transfer(pc, insn),
            0b10010 | 0b10011 => self.thumb_stack_pointer_transfer(pc, insn),
            0b10100 | 0b10101 => self.thumb_add_base(insn),
            0b10110 => self.thumb_stack_adjust_or_push(pc, insn),
            0b10111 => self.thumb_pop_or_armv6(pc, insn),
            0b11000 | 0b11001 => self.thumb_block_transfer(pc, insn),
            0b11010 | 0b11011 => self.thumb_branch_conditional_or_svc(pc, insn),
            0b11100 => self.thumb_branch(pc, insn),
            0b11110 => self.thumb_pair_branch(pc, insn),
            _ => Err(self.unimplemented(pc, insn, "Thumb-2 or ARMv6+")),
        }
    }

    /// Format 1: `LSLS`/`LSRS`/`ASRS Rd,Rs,#imm5`, of which a zero `LSLS` amount is `MOVS`.
    fn thumb_shift_by_immediate(&mut self, insn: u32) -> Result<(), Stop> {
        let kind = match insn >> 11 {
            0b00000 => Shift::Lsl,
            0b00001 => Shift::Lsr,
            _ => Shift::Asr,
        };
        let (out, carry) = shift_imm(
            self.reg((insn >> 3) & 7),
            kind,
            (insn >> 6) & 0x1f,
            self.carry(),
        );
        self.put(insn & 7, out, true, carry)
    }

    /// Format 2: `ADDS`/`SUBS` on a register or a three-bit immediate. Every form sets the flags
    /// and the operands are confined to `r0-r7`, so no R15 bias question arises.
    fn thumb_add_subtract(&mut self, insn: u32) -> Result<(), Stop> {
        let rd = insn & 7;
        let a = self.reg((insn >> 3) & 7);
        let b = match (insn >> 9) & 3 {
            0b00 | 0b01 => self.reg((insn >> 6) & 7),
            _ => (insn >> 6) & 7,
        };
        match (insn >> 9) & 3 {
            0b00 | 0b10 => self.summation(Some(rd), a, b, 0, true),
            _ => self.subtraction(Some(rd), a, b, 0, true),
        }
    }

    /// Format 2b: `ADDS`/`SUBS Rdn,#imm8`.
    ///
    /// The halfwords in this group mean two different things to two different architectures, and
    /// an ARMv4T core would read `0x300c` as `ADD r0,PC,#0xc`. `helloworld.mod` decides between
    /// them: `0x1ac`-`0x1c0` is
    ///
    /// ```text
    /// 2e20  cmps r6, 0x20
    /// d200  bcs  0x1b2
    /// 3620  adds r6, 0x20
    /// 481f  ldr  r0, [pc, 0x7c]      ; the module's data area
    /// 4478  add  r0, pc
    /// 6bc0  ldr  r0, [r0, 0x3c]      ; AEEModGenPool
    /// 6e81  ldr  r1, [r0, 0x68]      ; AEEHelperFuncs.malloc
    /// 1c30  adds r0, r6, 0           ; r0 = the requested size
    /// 300c  adds r0, 0xc             ; plus the header
    /// 4788  blx  r1                  ; malloc(size + 12)
    /// ```
    ///
    /// `0x3620` rounds a size up past `0x20` and `0x300c` adds `0xc` to the value `r6` was just
    /// copied out of — as the PC form they would hand `malloc` a code address and `AEEApplet_New`
    /// would fail on every handset, which is not a reading of a working sample. So this is the
    /// ARMv6.1 form, and the operands are `Rdn = Rdn ± imm8`.
    fn thumb_add_sub_immediate(&mut self, insn: u32) -> Result<(), Stop> {
        let rd = (insn >> 8) & 7;
        let a = self.reg(rd);
        let imm = insn & 0xff;
        if insn & bit(11) == 0 {
            self.summation(Some(rd), a, imm, 0, true)
        } else {
            self.subtraction(Some(rd), a, imm, 0, true)
        }
    }

    /// Format 3: `MOVS Rd,#imm8` and `CMPS Rd,#imm8`.
    fn thumb_move_or_compare_immediate(&mut self, insn: u32) -> Result<(), Stop> {
        let rd = (insn >> 8) & 7;
        let imm = insn & 0xff;
        if insn & bit(11) == 0 {
            // `MOVS` leaves the carry alone, which is why the carry handed to `put` is the
            // current one rather than something derived from the immediate.
            self.put(rd, imm, true, self.carry())
        } else {
            let a = self.reg(rd);
            self.subtraction(None, a, imm, 0, true)
        }
    }

    /// Formats 4, 5 and 6: the ALU operations, the high-register operations, `BX` and `BLX`.
    fn thumb_alu_or_branch(&mut self, insn: u32) -> Result<(), Stop> {
        let op = (insn >> 6) & 0x1f;
        if op < 0b10000 {
            return self.thumb_alu(insn, op);
        }
        // Above 15 the same two bit positions become the register-extension bits: bit 7 widens
        // the destination field and bit 6 the source field, which is what makes `r8`-`r15`
        // reachable. For `BX`/`BLX` the pair is a single four-bit `Rm` instead.
        let rd = (insn & 7) | ((insn >> 4) & 8);
        let rm = (insn >> 3) & 0xf;
        match (insn >> 8) & 7 {
            0b100 => {
                let out = self.reg(rd).wrapping_add(self.reg(rm));
                self.set_reg(rd, out);
                Ok(())
            }
            0b101 => {
                let (a, b) = (self.reg(rd), self.reg(rm));
                self.subtraction(None, a, b, 0, true)
            }
            0b110 => {
                let value = self.reg(rm);
                self.set_reg(rd, value);
                Ok(())
            }
            _ => {
                let target = self.reg(rm);
                if insn & bit(7) != 0 {
                    // `BLX` register: the return address is the next halfword, marked Thumb.
                    self.r[14] = self.pc | 1;
                }
                self.branch_to(target, true);
                Ok(())
            }
        }
    }

    /// Format 4. Thumb has no flag-free data processing, so every operation sets N and Z, and
    /// the logical forms simply leave C and V where they were.
    fn thumb_alu(&mut self, insn: u32, op: u32) -> Result<(), Stop> {
        let rd = insn & 7;
        let carry = self.carry();
        // The destructive two-operand form: the destination is also the first source operand, so
        // only `Rm` is named separately. Reading the second operand from `bits[8:6]` instead,
        // which is still part of the opcode field, is what the first draft of this did, and it
        // decodes `mvns r6, r2` as `mvns r6, r7`.
        let a = self.reg(rd);
        let b = self.reg((insn >> 3) & 7);
        match op {
            0b0000 => self.thumb_write_flags(rd, a & b),
            0b0001 => self.thumb_write_flags(rd, a ^ b),
            0b0010 | 0b0011 | 0b0100 | 0b0111 => {
                let kind = match op {
                    0b0010 => Shift::Lsl,
                    0b0011 => Shift::Lsr,
                    0b0100 => Shift::Asr,
                    _ => Shift::Ror,
                };
                let (out, carry) = shift_reg(a, kind, b & 0xff, carry);
                self.put(rd, out, true, carry)
            }
            0b0101 => self.summation(Some(rd), a, b, carry as u32, true),
            // `SBCS` subtracts the *inverse* of the carry, matching `ARM`'s `SBC`.
            0b0110 => self.subtraction(Some(rd), a, b, !carry as u32 & 1, true),
            0b1000 => {
                self.set_nz(a & b);
                Ok(())
            }
            0b1001 => self.subtraction(Some(rd), 0, b, 0, true),
            0b1010 => self.subtraction(None, a, b, 0, true),
            0b1011 => self.summation(None, a, b, 0, true),
            0b1100 => self.thumb_write_flags(rd, a | b),
            0b1101 => {
                // `MULS` sets N and Z only; C is left alone because ARMv5TE leaves it
                // unspecified here, and no module can rely on it.
                let out = a.wrapping_mul(b);
                self.set_nz(out);
                self.set_reg(rd, out);
                Ok(())
            }
            0b1110 => self.thumb_write_flags(rd, a & !b),
            _ => self.thumb_write_flags(rd, !b),
        }
    }

    fn thumb_write_flags(&mut self, rd: u32, value: u32) -> Result<(), Stop> {
        self.set_nz(value);
        self.set_reg(rd, value);
        Ok(())
    }

    /// `LDR Rd,[PC,#imm8*4]`. The literal pool lives past the code, so this is how a Thumb
    /// module reaches its constants — and the base is `(instruction + 4)` with the low two bits
    /// cleared, which matters at the many offsets where that address is not itself aligned.
    fn thumb_load_literal(&mut self, pc: u32, insn: u32) -> Result<(), Stop> {
        let address = (self.reg(15) & !3).wrapping_add((insn & 0xff) * 4);
        let value = self.load(pc, address, 4)?;
        self.set_reg((insn >> 8) & 7, value);
        Ok(())
    }

    /// Format 7: register-offset transfers. `bits[10:9]` is the width, `bit11` the direction —
    /// except in the `11` slot, where both encodings are loads.
    fn thumb_register_offset_transfer(&mut self, pc: u32, insn: u32) -> Result<(), Stop> {
        let address = self
            .reg((insn >> 3) & 7)
            .wrapping_add(self.reg((insn >> 6) & 7));
        let rd = insn & 7;
        let load = insn & bit(11) != 0;
        match (insn >> 9) & 3 {
            0b00 => self.thumb_transfer(pc, load, rd, address, 4, false),
            0b01 => self.thumb_transfer(pc, load, rd, address, 2, false),
            0b10 => self.thumb_transfer(pc, load, rd, address, 1, false),
            _ => {
                // The `0b11` width has no store form: the direction bit selects `LDRSB`
                // (bit11 clear) over `LDRSH` (bit11 set), so this arm is always a load.
                let size = if load { 2 } else { 1 };
                self.thumb_transfer(pc, true, rd, address, size, true)
            }
        }
    }

    /// Formats 8 and 9: immediate-offset transfers, where `bit12` separates the byte forms and
    /// halfwords get a group of their own.
    fn thumb_immediate_offset_transfer(&mut self, pc: u32, insn: u32) -> Result<(), Stop> {
        let halfword = matches!(insn >> 11, 0b10000 | 0b10001);
        let (size, scale) = if halfword {
            (2, 2)
        } else if insn & bit(12) != 0 {
            (1, 1)
        } else {
            (4, 4)
        };
        let address = self
            .reg((insn >> 3) & 7)
            .wrapping_add(((insn >> 6) & 0x1f) * scale);
        self.thumb_transfer(pc, insn & bit(11) != 0, insn & 7, address, size, false)
    }

    /// Formats 12 and 13: `STR`/`LDR Rd,[SP,#imm8*4]`, the frame-relative accesses a compiled
    /// local variable turns into. Here `Rd` sits above the immediate, as in `LDR` literal.
    fn thumb_stack_pointer_transfer(&mut self, pc: u32, insn: u32) -> Result<(), Stop> {
        let address = self.r[13].wrapping_add((insn & 0xff) * 4);
        self.thumb_transfer(pc, insn & bit(11) != 0, (insn >> 8) & 7, address, 4, false)
    }

    /// The shared body of the four transfer formats. `signed` is what distinguishes
    /// `LDRSB`/`LDRSH` from their unsigned siblings.
    fn thumb_transfer(
        &mut self,
        pc: u32,
        load: bool,
        rd: u32,
        address: u32,
        size: u32,
        signed: bool,
    ) -> Result<(), Stop> {
        if !load {
            let value = if rd == 15 {
                self.pc + 2
            } else {
                self.r[rd as usize]
            };
            return self.store(pc, address, size, value);
        }
        let raw = self.load(pc, address, size)?;
        let value = match (size, signed) {
            (1, true) => ((raw as u8) as i8) as u32,
            (2, true) => ((raw as u16) as i16) as u32,
            _ => raw,
        };
        self.set_reg(rd, value);
        Ok(())
    }

    /// Format 14: `ADD Rd,PC/SP,#imm8*4`, the other half of a compiler-generated `ADR`. The
    /// destination sits *above* the eight-bit immediate, and the group bit itself selects the
    /// base — `10100` is PC (which a reference decoder prints as `adr`), `10101` is SP. Like `LDR`
    /// literal and unlike the high-register `ADD`, the PC base is aligned and no flag is set.
    fn thumb_add_base(&mut self, insn: u32) -> Result<(), Stop> {
        let base = if insn & bit(11) != 0 {
            self.r[13]
        } else {
            self.reg(15) & !3
        };
        let out = base.wrapping_add((insn & 0xff) * 4);
        self.set_reg((insn >> 8) & 7, out);
        Ok(())
    }

    /// `0xB000`-`0xB0FF` moves SP by a multiple of four, `0xB200`-`0xB2FF` is the ARMv6 extend
    /// group, `0xB400`-`0xB5FF` is `PUSH`. Everything else in the group — `cbz`, the high-register
    /// `PUSH`, `rev` — belongs to a later architecture and is refused.
    fn thumb_stack_adjust_or_push(&mut self, pc: u32, insn: u32) -> Result<(), Stop> {
        if insn & 0xff00 == 0xb000 {
            let amount = (insn & 0x7f) * 4;
            self.r[13] = if insn & bit(7) != 0 {
                self.r[13].wrapping_sub(amount)
            } else {
                self.r[13].wrapping_add(amount)
            };
            return Ok(());
        }
        // `SXTH`/`SXTB`/`UXTH`/`UXTB`: bits 7-6 pick the pair and the width. A pack built for an
        // ARMv6 handset uses them on its start-up path, and the ARM decoder already computes the
        // same four — this is the one decoder gap, not a new operation.
        if insn & 0xff00 == 0xb200 {
            let rd = insn & 7;
            let rm = (insn >> 3) & 7;
            let (signed, bits) = match (insn >> 6) & 3 {
                0 => (true, 16),
                1 => (true, 8),
                2 => (false, 16),
                _ => (false, 8),
            };
            return self.extend_register(rd, rm, signed, bits);
        }
        if insn & 0xfe00 == 0xb400 {
            return self.thumb_push(pc, insn);
        }
        Err(self.unimplemented(pc, insn, "ARMv6+ stack/extend"))
    }

    /// `PUSH {r0-r7,lr}`. Full-descending by definition, so SP moves first.
    fn thumb_push(&mut self, pc: u32, insn: u32) -> Result<(), Stop> {
        let list = insn & 0xff;
        let with_lr = insn & bit(8) != 0;
        if list == 0 && !with_lr {
            return Err(self.illegal(pc, insn));
        }
        let count = list.count_ones() + u32::from(with_lr);
        let sp = self.r[13];
        let mut address = sp.wrapping_sub(count * 4);
        // Walk the set bits instead of the eight slots: `PUSH {r4,r5,lr}` used to test eight positions
        // to store three, and the ascending order `trailing_zeros` gives is the same order the range
        // walked. `rest` is a copy — `list` is the binding the count above and the caller still need.
        let mut rest = list;
        while rest != 0 {
            let reg = rest.trailing_zeros();
            rest &= rest - 1;
            self.store(pc, address, 4, self.r[reg as usize])?;
            address = address.wrapping_add(4);
        }
        if with_lr {
            self.store(pc, address, 4, self.r[14])?;
        }
        self.r[13] = sp.wrapping_sub(count * 4);
        Ok(())
    }

    /// `0xBC00`-`0xBDFF` is `POP`; the rest of the group is ARMv6+ (`udf`, `cbnz`, `rev`,
    /// `bkpt`, `it`) or Thumb-2. Only bit 8 varies within `POP`, so a wider mask would let
    /// `bkpt` and `it` through as a stack pop.
    fn thumb_pop_or_armv6(&mut self, pc: u32, insn: u32) -> Result<(), Stop> {
        if insn & 0xfe00 == 0xbc00 {
            return self.thumb_pop(pc, insn);
        }
        Err(self.unimplemented(pc, insn, "ARMv6+ stack/hint/IT"))
    }

    /// `POP {r0-r7[,pc]}`. A popped PC is an interworking return, which is how every Thumb
    /// function in these modules actually returns.
    fn thumb_pop(&mut self, pc: u32, insn: u32) -> Result<(), Stop> {
        let list = insn & 0xff;
        let with_pc = insn & bit(8) != 0;
        if list == 0 && !with_pc {
            return Err(self.illegal(pc, insn));
        }
        let mut address = self.r[13];
        let sp = self.r[13];
        let mut rest = list;
        while rest != 0 {
            let reg = rest.trailing_zeros();
            rest &= rest - 1;
            let value = self.load(pc, address, 4)?;
            self.r[reg as usize] = value;
            address = address.wrapping_add(4);
        }
        let count = list.count_ones() + u32::from(with_pc);
        self.r[13] = sp.wrapping_add(count * 4);
        if with_pc {
            let value = self.load(pc, address, 4)?;
            self.branch_to(value, true);
        }
        Ok(())
    }

    /// Formats 18 and 19: `STMIA`/`LDMIA`. Both write back, which is why Thumb `PUSH`/`POP` and
    /// these two share the "base always moves" shape — with one exception the sample set found: a *load*
    /// whose own base register sits in the register list leaves that register holding the word it
    /// loaded, which `a_load_whose_base_is_in_its_own_list_keeps_the_loaded_word` pins down.
    fn thumb_block_transfer(&mut self, pc: u32, insn: u32) -> Result<(), Stop> {
        let load = insn & bit(11) != 0;
        let rb = (insn >> 8) & 7;
        let list = insn & 0xff;
        if list == 0 {
            return Err(self.illegal(pc, insn));
        }
        let base = self.r[rb as usize];
        let count = list.count_ones();
        let mut address = base;
        let mut rest = list;
        while rest != 0 {
            let reg = rest.trailing_zeros();
            rest &= rest - 1;
            if load {
                let value = self.load(pc, address, 4)?;
                self.r[reg as usize] = value;
            } else {
                let value = self.r[reg as usize];
                self.store(pc, address, 4, value)?;
            }
            address = address.wrapping_add(4);
        }
        if !load || list & bit(rb) == 0 {
            self.r[rb as usize] = base.wrapping_add(count * 4);
        }
        Ok(())
    }

    /// Format 20: `B<cond>` over a nine-bit signed offset, plus the `0xDFxx` `SVC` slot that
    /// shares the group. `0xDExxx` is ARMv6 `UDF` and is refused with the condition name.
    fn thumb_branch_conditional_or_svc(&mut self, pc: u32, insn: u32) -> Result<(), Stop> {
        let cond = (insn >> 8) & 0xf;
        if cond >= 14 {
            if cond == 15 {
                // Not the HLE seam. A host call is an `SVC` in *ARM* state whose 24-bit immediate
                // carries `(iface << 12) | slot`; eight bits cannot hold one, and dispatching on
                // the truncated value would resolve a call to a plausible but wrong slot.
                return Err(self.unimplemented(pc, insn, "Thumb SVC (too few id bits)"));
            }
            return Err(self.unimplemented(pc, insn, "ARMv6+ UDF"));
        }
        if !condition_met(self, cond) {
            return Ok(());
        }
        let offset = sign_extend(insn & 0xff, 8) * 2;
        self.pc = (pc + 4).wrapping_add(offset as u32);
        Ok(())
    }

    /// Format 21: the unconditional `B`, whose eleven-bit offset reaches the whole ±2 KiB a
    /// function needs.
    fn thumb_branch(&mut self, pc: u32, insn: u32) -> Result<(), Stop> {
        let offset = sign_extend(insn & 0x7ff, 11) * 2;
        self.pc = (pc + 4).wrapping_add(offset as u32);
        Ok(())
    }

    /// The `BL`/`BLX` pair, the one 32-bit instruction of Thumb-1.
    ///
    /// The first halfword carries `S` and the top ten bits of the offset, the second carries the
    /// low eleven plus the bit that says whether the callee is Thumb (`BL`, staying in state) or
    /// ARM (`BLX`, dropping out of it). The offset is the concatenation scaled by four: eleven
    /// bits shifted by twelve, plus eleven bits shifted by one, relative to `instruction + 4`.
    /// That is the formula the reference decoder applies — it reproduces the target of every pair
    /// probed, including the module's own `0xf7ff:0xede4`, which is `-1080` from its slot.
    fn thumb_pair_branch(&mut self, pc: u32, insn: u32) -> Result<(), Stop> {
        let high = self.load(pc, pc + 2, 2)?;
        // `111 L 1 imm11`: bit 12 is the direction bit and is deliberately outside the mask, so
        // demanding it be clear here would fault on every plain Thumb `bl`.
        if high & 0xe800 != 0xe800 {
            return Err(self.illegal(pc, insn << 16 | high));
        }
        // `S:imm10` carries the sign; the low eleven bits are unsigned, which is why the two
        // halves are not extended the same way.
        let offset = (sign_extend(insn & 0x7ff, 11) << 12) + i64::from(high & 0x7ff) * 2;
        let destination = (pc + 4).wrapping_add(offset as u32);
        // The caller comes back to the halfword after the pair, in Thumb state.
        self.r[14] = (pc + 4) | 1;
        if high & bit(12) == 0 {
            self.set_thumb(false);
            self.pc = destination & !3;
        } else {
            self.pc = destination & !1;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "../test/cpu/thumb.rs"]
mod tests;
