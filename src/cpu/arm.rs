//! ARM-state (32-bit) decode and execute.
//!
//! Coverage is the ARMv5TE subset a RVCT-built BREW module actually emits. Where a group is
//! defined but not implemented the executor reports it by name instead of inventing a semantic,
//! so `--trace` points straight at the gap. Every encoding here was derived either from the
//! bytes of a delivered module or an independent inspection of a hand-assembled word; a
//! hand-assembler was tried as the other record and is not trustworthy (it mis-encodes
//! BLX-immediate, the halfword transfers and SWP), so it is not the source of any claim below.
//!
//! Refused by name rather than guessed at: the coprocessor group, multiply-shaped words this core
//! has not identified, and the halfword group's writeback (`*T`) forms, none of which a module built
//! by RVCT has been seen to execute.

use super::{shift_imm, shift_reg, Cpu, Shift, Stop, FLAG_C, FLAG_N, FLAG_T, FLAG_V, FLAG_Z};

pub const CONDITION_NAMES: [&str; 16] = [
    "eq", "ne", "cs", "cc", "mi", "pl", "vs", "vc", "hi", "ls", "ge", "lt", "gt", "le", "al", "nv",
];

#[inline(always)]
pub fn condition_met(cpu: &Cpu, cond: u32) -> bool {
    let n = cpu.flag(FLAG_N);
    let z = cpu.flag(FLAG_Z);
    let c = cpu.flag(FLAG_C);
    let v = cpu.flag(FLAG_V);
    match cond & 0xf {
        0 => z,
        1 => !z,
        2 => c,
        3 => !c,
        4 => n,
        5 => !n,
        6 => v,
        7 => !v,
        8 => c && !z,
        9 => !c || z,
        10 => n == v,
        11 => n != v,
        12 => !z && (n == v),
        13 => z || (n != v),
        14 => true,
        _ => false,
    }
}

const fn bit(n: u32) -> u32 {
    1 << n
}

/// Signed 24-bit branch/offset field scaled by four: a 26-bit signed byte offset.
fn scaled_offset(insn: u32) -> u32 {
    let raw = (insn & 0x00ff_ffff) << 2;
    if insn & bit(23) != 0 {
        raw.wrapping_add(0xfc00_0000)
    } else {
        raw
    }
}

/// `SXTB`/`SXTH`/`UXTB`/`UXTH`, returned as `(Rd, Rm, signed, source bits)`.
///
/// Two fixed fields tell them from everything else in the load/store group — `bits[19:16]` are
/// `1111` and `bits[11:4]` are `0x07` — so no media-space word can be mistaken for one of these by
/// sharing a neighbour's bits.
const fn extend_operands(insn: u32) -> Option<(u32, u32, bool, u32)> {
    if (insn >> 16) & 0xf != 0xf || (insn >> 4) & 0xff != 0x07 {
        return None;
    }
    let (signed, bits) = match (insn >> 20) & 0xff {
        0x6a => (true, 8),
        0x6b => (true, 16),
        0x6e => (false, 8),
        0x6f => (false, 16),
        _ => return None,
    };
    Some(((insn >> 12) & 0xf, insn & 0xf, signed, bits))
}

impl Cpu {
    pub(crate) fn step_arm(&mut self, pc: u32, insn: u32) -> Result<(), Stop> {
        // ARMv5TE leaves condition `0b1111` UNDEFINED — it is not "always", which only came back in
        // ARMv8. The space that prefix opens is not all undefined, though: ARMv5 put BLX-immediate
        // there for ARM→Thumb interworking, and guest modules emit it. Executing the rest as a nop
        // instead lets a guest that has branched into a data blob slide through whole `0xffffffff`
        // words and abort far from the branch that left the code, so everything else still faults.
        if insn >> 28 == 0b1111 {
            if insn & 0xfe00_0000 == 0xfa00_0000 {
                return self.arm_blx_immediate(insn);
            }
            return Err(self.illegal(pc, insn));
        }
        // Most ARM code is `AL`: skip the flag-decoding condition test entirely for it.
        if insn >> 28 != 0b1110 && !condition_met(self, insn >> 28) {
            return Ok(());
        }
        // `SVC` is the HLE seam: everything the guest asks the runtime for arrives here.
        if insn & 0x0f00_0000 == 0x0f00_0000 {
            return Err(self.supervisor(pc, insn & 0x00ff_ffff));
        }
        // bits[27:25] is the group. Masking matters: the field is only three bits wide and the
        // condition code sits directly above it.
        match (insn >> 25) & 0b111 {
            0b000 | 0b001 => self.arm_dp_muls(pc, insn),
            0b010 | 0b011 => {
                // ARMv6's sign/zero extends put themselves in this group's bits[27:25] but in none
                // of its fields: bits[19:16] are fixed `1111` and bits[11:4] fixed `0x07`. Read as a
                // transfer — which is what this host did — the instruction *ran* and changed
                // nothing, because `Rn` came out as r15 and the write-back through PC is skipped. A
                // guest pack's `sxth` (`a sample module`, `a sample module`) therefore executed as a no-op with
                // its effect pushed 600 000 instructions down the road, which is the failure shape
                // this file's own rule forbids: a value that looks reasonable instead of a refusal.
                if let Some((rd, rm, signed, bits)) = extend_operands(insn) {
                    return self.extend_register(rd, rm, signed, bits);
                }
                self.arm_transfer(pc, insn)
            }
            0b100 => self.arm_block_transfer(pc, insn),
            0b101 => self.arm_branch(insn),
            // 0b110 is LDC/STC, 0b111 is CDP/MRC/MCR. An ARM1136J-S with no VFP still has the
            // generic coprocessor interface, but nothing a BREW module executes uses it.
            _ => Err(self.unimplemented(pc, insn, "coprocessor")),
        }
    }

    /// Groups `000`/`001`: data processing, moves, multiplies, CLZ, MRS/MSR.
    #[inline(always)]
    fn arm_dp_muls(&mut self, pc: u32, insn: u32) -> Result<(), Stop> {
        // Only group `000` holds the multiply/BX/halfword forms. Group `001` is a data-processing
        // *immediate*, whose bit4 is simply a bit of `imm8` — `mov r0, #0xf0` is `e3a000f0`.
        //
        // `bit4` is not enough to identify the special families: in a register-operand
        // data-processing instruction it means "shift by a register", so every variable shift a
        // compiler emits carries it too. The special path therefore ends by handing the word back.
        if insn & bit(25) == 0 && insn & bit(4) != 0 {
            return self.arm_special_or_multiply(pc, insn);
        }
        self.arm_dp_operand(insn)
    }

    /// The data-processing executor for both operand forms: `bit25` selects immediate data, and in
    /// the register form `bit4` selects whether the shift amount comes from a register or from the
    /// instruction.
    fn arm_dp_operand(&mut self, insn: u32) -> Result<(), Stop> {
        let rd = (insn >> 12) & 0xf;
        let rn = (insn >> 16) & 0xf;
        let opcode = (insn >> 21) & 0xf;
        let s = insn & bit(20) != 0;

        let (operand2, carry_out) = if insn & bit(25) != 0 {
            let imm8 = insn & 0xff;
            let rot = (insn >> 8) & 0xf;
            if rot == 0 {
                (imm8, imm8 & bit(7) != 0)
            } else {
                (
                    imm8.rotate_right(rot * 2),
                    imm8.rotate_right(rot * 2) >> 31 != 0,
                )
            }
        } else {
            let kind = match (insn >> 5) & 3 {
                0 => Shift::Lsl,
                1 => Shift::Lsr,
                2 => Shift::Asr,
                _ => Shift::Ror,
            };
            let value = self.reg(insn & 0xf);
            if insn & bit(4) != 0 {
                // Named through the accessor: the field is four bits wide and the register file holds
                // fifteen, so Rs=15 would index past its end. See the same rule at `arm_dp_muls`.
                let amount = self.reg((insn >> 8) & 0xf) & 0xff;
                shift_reg(value, kind, amount, self.carry())
            } else {
                shift_imm(value, kind, (insn >> 7) & 0x1f, self.carry())
            }
        };

        let a = self.reg(rn);
        let not_carry = !self.carry() as u32 & 1;
        match opcode {
            0b0000 => self.put(rd, a & operand2, s, carry_out),
            0b0001 => self.put(rd, a ^ operand2, s, carry_out),
            0b0010 => self.subtraction(Some(rd), a, operand2, 0, s), // SUB
            0b0011 => self.subtraction(Some(rd), operand2, a, 0, s), // RSB
            0b0100 => self.summation(Some(rd), a, operand2, 0, s),   // ADD
            0b0101 => self.summation(Some(rd), a, operand2, self.carry() as u32, s), // ADC
            0b0110 => self.subtraction(Some(rd), a, operand2, not_carry, s), // SBC
            0b0111 => self.subtraction(Some(rd), operand2, a, not_carry, s), // RSC
            0b1000 => {
                self.set_nz(a & operand2);
                self.set_flag(FLAG_C, carry_out);
                Ok(())
            }
            0b1001 => {
                self.set_nz(a ^ operand2);
                self.set_flag(FLAG_C, carry_out);
                Ok(())
            }
            0b1010 => self.subtraction(None, a, operand2, 0, true), // CMP
            0b1011 => self.summation(None, a, operand2, 0, true),   // CMN
            0b1100 => self.put(rd, a | operand2, s, carry_out),
            0b1101 => self.put(rd, operand2, s, carry_out), // MOV
            0b1110 => self.put(rd, a & !operand2, s, carry_out), // BIC
            _ => self.put(rd, !operand2, s, carry_out),     // MVN
        }
    }

    /// The `[3:0]=1` neighbourhood inside group `000`: BX, BLX, CLZ, MRS/MSR, multiplies.
    fn arm_special_or_multiply(&mut self, pc: u32, insn: u32) -> Result<(), Stop> {
        let top = insn & 0x0fff_fff0;
        if top == 0x012f_ff10 || top == 0x012f_ff20 || top == 0x012f_ff30 {
            let target = self.reg(insn & 0xf);
            if top == 0x012f_ff30 {
                self.r[14] = self.pc;
            }
            self.branch_to(target, true);
            return Ok(());
        }
        if top == 0x0120_0070 {
            return Err(self.illegal(pc, insn)); // BKPT with no debugger attached.
        }
        // CLZ carries its destination in bits 15..12, so the mask has to leave Rd free. With
        // `0x0fff_fff0` the comparison pinned Rd to 1, and every `clz` with another destination
        // fell through to the multiply group — where it was decoded as something else and left
        // the register untouched. A 64-bit division routine normalises its divisor with
        // `clz r6, r2` and then shifts by that count, so a silent zero there turns the whole
        // quotient into nonsense rather than raising anything.
        if insn & 0x0fff_0ff0 == 0x016f_0f10 {
            // CLZ r6, r2 -> e16f6f12
            let value = self.reg(insn & 0xf);
            self.set_reg((insn >> 12) & 0xf, value.leading_zeros());
            return Ok(());
        }
        if insn & 0x0fbf_0000 == 0x010f_0000 && insn & 0xfff == 0 {
            // MRS r3, apsr -> e10f3000
            let from_spsr = insn & bit(22) != 0;
            let value = if from_spsr { self.spsr } else { self.cpsr };
            // Only the flags and the low mode/control byte are visible to a read.
            self.set_reg(
                (insn >> 12) & 0xf,
                value & (FLAG_N | FLAG_Z | FLAG_C | FLAG_V | FLAG_T | 0xff),
            );
            return Ok(());
        }
        if insn & 0x0fa0_f0ff == 0x0120_0000 {
            // MSR apsr_nzcvqg, r0 -> e12cf000
            let mask_field = (insn >> 16) & 0xf;
            let value = self.reg(insn & 0xf);
            let to_spsr = insn & bit(22) != 0;
            let mut byte_mask = 0u32;
            if mask_field & 1 != 0 {
                byte_mask |= 0x0000_00ff;
            }
            if mask_field & 2 != 0 {
                byte_mask |= 0x0000_ff00;
            }
            if mask_field & 4 != 0 {
                byte_mask |= 0x00ff_0000;
            }
            if mask_field & 8 != 0 {
                byte_mask |= 0xff00_0000;
            }
            let current = if to_spsr { self.spsr } else { self.cpsr };
            let updated = (current & !byte_mask) | (value & byte_mask & !FLAG_T);
            if to_spsr {
                self.spsr = updated;
            } else {
                self.cpsr = (self.cpsr & FLAG_T) | (updated & !FLAG_T);
            }
            return Ok(());
        }

        // The halfword group. Bits[7:4]=0xb is `LDRH`/`STRH` with an immediate offset — the
        // instruction a real guest module faulted on, `0xe1c405b0` = `strh r0, [r4, #0x50]` — and
        // its 0xd/0xf neighbours are the signed loads `LDRSB`/`LDRSH`, all four confirmed against
        // an independent inspection. With bit20 clear those same two nibbles
        // are instead the pair transfers `LDRD`/`STRD`, which is what stopped two store titles.
        // What is left in this bit pattern is refused by name: an even nibble in bits[7:4] is not
        // one of the four transfer forms above, and this core has not identified it.
        if matches!((insn >> 4) & 0xf, 0xb | 0xd | 0xf) {
            return self.arm_halfword(pc, insn);
        }
        if insn & bit(7) != 0 && insn & (bit(6) | bit(5)) != 0 {
            return Err(self.unimplemented(pc, insn, "halfword transfer"));
        }

        let accumulate = insn & bit(21) != 0;
        let s = insn & bit(20) != 0;
        // The multiply group numbers its registers differently from data processing, which is easy
        // to get wrong and impossible to notice from a test that was written against the same
        // mistake: `Rd` is bits[19:16], and bits[15:12] is the accumulator — required to be r0 for
        // `MUL`, which is how a guest module's `mul r2, r5, r10` (`0xe002059a`) confirms the order
        // from both directions. Only the long forms pair the two fields, as `RdLo`/`RdHi`.
        let rd = (insn >> 16) & 0xf;
        let rn = (insn >> 12) & 0xf;
        let rs = (insn >> 8) & 0xf;
        let rm = insn & 0xf;
        // Named through the accessors rather than indexed: the two operand fields of a multiply and
        // the two destination fields of a long one are all `0..=15`, and a word that only *looks*
        // like a multiply reaches this code on its way to the data-processing arm below — where
        // `self.r[15]` would be an index past the end of the register file rather than a refusal.
        let (x, y) = (self.reg(rm), self.reg(rs));
        match insn & 0x0fc0_00f0 {
            0x0000_0090 => {
                let mut out = x.wrapping_mul(y);
                if accumulate {
                    out = out.wrapping_add(self.reg(rn));
                }
                self.set_reg(rd, out);
                if s {
                    self.set_nz(out);
                }
                Ok(())
            }
            0x0040_0090 => Err(self.unimplemented(pc, insn, "umaal")),
            group @ (0x0080_0090 | 0x00c0_0090) => {
                let signed = group & 0x0040_0000 != 0;
                let product = if signed {
                    (x as i32 as i64 as u64).wrapping_mul(y as i32 as i64 as u64)
                } else {
                    u64::from(x) * u64::from(y)
                };
                let (rdlo, rdhi) = (rn, rd);
                let out = if accumulate {
                    let acc = u64::from(self.reg(rdlo)) | (u64::from(self.reg(rdhi)) << 32);
                    acc.wrapping_add(product)
                } else {
                    product
                };
                self.set_reg(rdlo, out as u32);
                self.set_reg(rdhi, (out >> 32) as u32);
                if s {
                    // Hardware leaves C and V alone (measured with both preset: `umulls`/`smlals`
                    // return `C=1 V=1`), and sets N and Z from the *low* half — the product's high
                    // half being zero does not raise Z. This core still writes both from `RdHi` and
                    // clears C and V, which no sample set word has been measured to exercise; recorded
                    // rather than quietly kept.
                    self.set_nz((out >> 32) as u32);
                    self.set_flag(FLAG_C, false);
                    self.set_flag(FLAG_V, false);
                }
                Ok(())
            }
            // What reaches here has the multiply nibble `1001` in bits[7:4] but no combination of
            // bit24 and bits[23:21] this core has identified. Sweeping every word of all five
            // guest modules finds no such word executed anywhere, so there is no sample set evidence
            // to name it by and no way to check a guess — which is exactly why it stops by name
            // rather than being run as the data-processing instruction its nibble resembles.
            _ if insn & 0xf0 == 0x90 => Err(self.unimplemented(pc, insn, "unclassified multiply")),
            // Nothing special matched this word, so it is a data-processing instruction that
            // happened to arrive here because `bit4` is set — a shift by register. Measured over
            // 29 525 words of a guest module, the old catch-all (`bits[27:24] == 0001`, which is
            // also `MOV`'s shape) swallowed 903 words, and the binary contains no `QADD`, `QSUB` or
            // `SAT` at all; this is where `lsl r1, r0, r3` died.
            _ => self.arm_dp_operand(insn),
        }
    }

    #[inline(always)]
    pub(crate) fn put(&mut self, rd: u32, value: u32, s: bool, carry: bool) -> Result<(), Stop> {
        if s {
            self.set_nz(value);
            self.set_flag(FLAG_C, carry);
        }
        self.set_reg(rd, value);
        Ok(())
    }

    /// `rd` is `None` for the compare forms, which set flags and discard the result.
    pub(crate) fn summation(
        &mut self,
        rd: Option<u32>,
        a: u32,
        b: u32,
        cin: u32,
        s: bool,
    ) -> Result<(), Stop> {
        let wide = u64::from(a) + u64::from(b) + u64::from(cin);
        let out = wide as u32;
        if s {
            self.set_nz(out);
            self.set_flag(FLAG_C, wide > u64::from(u32::MAX));
            self.set_flag(FLAG_V, ((a ^ out) & (b ^ out)) >> 31 != 0);
        }
        if let Some(rd) = rd {
            self.set_reg(rd, out);
        }
        Ok(())
    }

    pub(crate) fn subtraction(
        &mut self,
        rd: Option<u32>,
        a: u32,
        b: u32,
        borrow: u32,
        s: bool,
    ) -> Result<(), Stop> {
        let exact = i64::from(a) - i64::from(b) - i64::from(borrow);
        let out = a.wrapping_sub(b).wrapping_sub(borrow);
        if s {
            self.set_nz(out);
            self.set_flag(FLAG_C, exact >= 0);
            self.set_flag(FLAG_V, ((a ^ b) & (a ^ out)) >> 31 != 0);
        }
        if let Some(rd) = rd {
            self.set_reg(rd, out);
        }
        Ok(())
    }

    /// `SXTB`/`SXTH`/`UXTB`/`UXTH`, shared by both instruction sets: the ARM encodings at
    /// `cond 0110 101x 1111 Rd 0000 0111 Rm` and Thumb's `0xB200`-`0xB2FF`.
    ///
    /// ARMv6, and outside the class this core otherwise models, but a guest pack executes them on
    /// its own module-init path (the ARM words from `a sample module`, the Thumb ones from a pack built for
    /// a v6 handset). Four instructions answered honestly beats the alternative the host had —
    /// decoding one as a transfer and letting it do nothing, or refusing the Thumb form outright —
    /// because a guest that reads the result cannot tell "not implemented" from "computed".
    pub(super) fn extend_register(
        &mut self,
        rd: u32,
        rm: u32,
        signed: bool,
        bits: u32,
    ) -> Result<(), Stop> {
        let raw = self.reg(rm);
        let up = 32 - bits;
        let out = if signed {
            (((raw << up) as i32) >> up) as u32
        } else {
            (raw << up) >> up
        };
        self.set_reg(rd, out);
        Ok(())
    }

    /// Groups `010`/`011`: LDR/STR, word and byte, immediate and register offset. The halfword,
    /// signed and doubleword forms are *not* here — they encode in group `000`.
    #[inline(always)]
    fn arm_transfer(&mut self, pc: u32, insn: u32) -> Result<(), Stop> {
        let i = insn & bit(25) == 0;
        let p = insn & bit(24) != 0;
        let u = insn & bit(23) != 0;
        let b = insn & bit(22) != 0;
        let mut w = insn & bit(21) != 0;
        let l = insn & bit(20) != 0;
        let rn = (insn >> 16) & 0xf;
        let rd = (insn >> 12) & 0xf;

        // The register-offset form carries a barrel shift of its own: `ldrblt ip, [ip, r1, asr #15]`
        // comes out of code a pack inflated into its heap, and decoding only LSL here faults that run.
        //
        // The shift's flags are deliberately dropped. The architecture text says a shifted register
        // offset updates N and (for a shift of at least one) C, but a pack's own inflate reads the
        // flags across an `LSL`-shifted transfer, so honouring that sentence takes a run from 53
        // presentations to a fault inside its own `memset` (a four-billion-byte length built out of the
        // garbage its inflate then produces). A pack that ran on real hardware outranks the sentence,
        // so the address is computed and the flags are left exactly as the condition found them.
        let offset = if i {
            insn & 0xfff
        } else {
            let kind = match (insn >> 5) & 3 {
                0 => Shift::Lsl,
                1 => Shift::Lsr,
                2 => Shift::Asr,
                _ => Shift::Ror,
            };
            let amount = (insn >> 7) & 0x1f;
            let (out, _) = shift_imm(self.reg(insn & 0xf), kind, amount, self.carry());
            out
        };
        let size = if b { 1 } else { 4 };
        let base = self.reg(rn);
        let address = if u {
            base.wrapping_add(offset)
        } else {
            base.wrapping_sub(offset)
        };
        let effective = if p { address } else { base };
        // Writing back through R15 is UNPREDICTABLE; leaving it alone is what a real core
        // presents to ROPI code, which never relies on it.
        if rn == 15 {
            w = false;
        }

        if l {
            let value = self.load(pc, effective, size)?;
            if rd == 15 {
                // `LDR pc, ...` is `LoadWritePC`, and from ARMv5 on that behaves as `BX` — bit 0 of
                // the loaded word selects the instruction set. It is how ARM code returns to a Thumb
                // caller: `pop {pc}` assembles to exactly this instruction (`e49df004`), and reading
                // it as an ordinary register write dropped the T bit, so the caller's Thumb bytes were
                // then fetched as ARM (a sample pack faulted on `f0 f7 a5 f9` at 0x1000fce0).
                self.branch_to(value, true);
            } else {
                self.set_reg(rd, value);
            }
        } else {
            // A stored R15 is the address of the *next* instruction plus four.
            let value = if rd == 15 {
                self.pc + 4
            } else {
                self.r[rd as usize]
            };
            self.store(pc, effective, size, value)?;
        }
        if !p || w {
            self.set_reg(rn, address);
        }
        Ok(())
    }

    /// The halfword group: `LDRH`, `STRH`, `LDRSB`, `LDRSH`, and the pair transfers `LDRD`/`STRD`
    /// that share the field layout. Both offset forms — the 8-bit immediate split across the
    /// instruction, and the register offset — reach here.
    ///
    /// The offset is split across the instruction — high nibble in bits[11:8], low in bits[3:0] —
    /// which is why `0xe1c405b0` is `[#0x50]` and not `[#0x5]`. Addressing behaves exactly as in the
    /// word group, including writeback and the R15 rules, because it is the same `P U - W L` field,
    /// and the pair transfers share all of it: only their data phase differs, in moving two registers
    /// and in taking their direction from bit5 rather than bit20.
    #[inline(always)]
    fn arm_halfword(&mut self, pc: u32, insn: u32) -> Result<(), Stop> {
        let nibble = (insn >> 4) & 0xf;
        let l = insn & bit(20) != 0;
        // `LDRD`/`STRD` share this group's field layout but not its direction bit: they arrive with
        // bit20 *clear* and bits[6:5] = `10`/`11` (nibbles `0xd` and `0xf`), and take load-versus-
        // store from **bit5** rather than from bit20. That is why the word `0xe1cd82d0` is
        // `ldrd r8, sb, [sp, #0x20]` while setting bit5 on it — moving the nibble to `0xf` — reads
        // the matching `strd`, both confirmed by two independent inspections of the same bytes.
        // The pair therefore has to be picked out ahead of the store side below, which reads bit20
        // as the direction the way the rest of the group does.
        let pair = !l && nibble != 0xb;
        let p = insn & bit(24) != 0;
        let u = insn & bit(23) != 0;
        // Bit22 selects the *offset form*, not the byte size as it does in the LDR/STR group: set
        // means the 8-bit offset is split across bits[11:8] and bits[3:0], clear means Rm names the
        // offset register and bits[11:8] are ignored (should be zero). Measured on the pair group
        // too: `0xe18d82d0` has bit22 clear and the register form reads `[sp, r0]`.
        let immediate = insn & bit(22) != 0;
        let mut w = insn & bit(21) != 0;
        let rn = (insn >> 16) & 0xf;
        let rd = (insn >> 12) & 0xf;

        let offset = if immediate {
            ((insn >> 4) & 0xf0) | (insn & 0xf)
        } else {
            self.reg(insn & 0xf)
        };
        let base = self.reg(rn);
        let address = if u {
            base.wrapping_add(offset)
        } else {
            base.wrapping_sub(offset)
        };
        let effective = if p { address } else { base };
        if rn == 15 {
            w = false;
        }

        if pair {
            // The pair moves `Rt` from the addressed word and `Rt + 1` from the one after it. An odd
            // `Rt`, or a second register past r14, is UNPREDICTABLE: no reference compiler emits one, so
            // it is named rather than answered with a result the architecture refuses to define.
            let store = insn & bit(5) != 0;
            let hi = rd + 1;
            if rd % 2 == 1 || hi > 14 {
                return Err(self.unimplemented(pc, insn, "unpredictable doubleword register pair"));
            }
            if store {
                self.store(pc, effective, 4, self.r[rd as usize])?;
                self.store(pc, effective + 4, 4, self.r[hi as usize])?;
            } else {
                let lo = self.load(pc, effective, 4)?;
                let up = self.load(pc, effective + 4, 4)?;
                self.set_reg(rd, lo);
                self.set_reg(hi, up);
            }
        } else if l {
            let half = self.load(pc, effective, 2)? as u16;
            let value = match nibble {
                0xb => u32::from(half),
                0xd => (half as i8) as i32 as u32,
                _ => (half as i16) as i32 as u32,
            };
            self.set_reg(rd, value);
        } else {
            // A stored R15 is the address of the *next* instruction plus four.
            let value = if rd == 15 {
                self.pc + 4
            } else {
                self.r[rd as usize]
            };
            self.store(pc, effective, 2, value)?;
        }
        if !p || w {
            self.set_reg(rn, address);
        }
        Ok(())
    }

    #[inline(always)]
    fn arm_block_transfer(&mut self, pc: u32, insn: u32) -> Result<(), Stop> {
        let p = insn & bit(24) != 0;
        let u = insn & bit(23) != 0;
        let _s = insn & bit(22) != 0;
        let mut w = insn & bit(21) != 0;
        let l = insn & bit(20) != 0;
        let rn = (insn >> 16) & 0xf;
        let list = insn & 0xffff;
        let base = self.reg(rn);
        let span = list.count_ones() * 4;
        // Here `p` is *before*: STMDB is P=1,U=0 and LDMIA (`pop`) is P=0,U=1, the inverse of what
        // the LDR/STR group means. The four modes cover the same ascending address set; only its
        // position and where the base ends up move.
        let first = match (u, p) {
            (true, true) => base.wrapping_add(4),
            (true, false) => base,
            (false, true) => base.wrapping_sub(span),
            (false, false) => base.wrapping_sub(span.wrapping_sub(4)),
        };
        let written_back = match (u, p) {
            (true, true) => base.wrapping_add(span),
            (true, false) => base.wrapping_add(span),
            (false, true) => base.wrapping_sub(span),
            (false, false) => base.wrapping_sub(span.wrapping_sub(4)),
        };
        if rn == 15 {
            w = false;
        }
        // Only `W` moves the base in this group — `!P` is Thumb's "always write back" rule and does
        // not belong here — and it happens *before* the transfers, because when the base register is
        // itself in the list the loaded value is what the register ends up holding. A guest module
        // reloads its own argument pointer exactly that way (`ldm r6, {r4,r5,r6}`, no writeback), and
        // taking the address instead handed it a stack address where a small index belonged.
        if w {
            self.set_reg(rn, written_back);
        }
        let mut addr = first;
        // The set bits, not the sixteen slots: `ldm r6, {r4,r5,r6}` used to test sixteen positions to
        // move three words. `trailing_zeros` walks ascending, which is the order the range walked.
        let mut rest = list;
        while rest != 0 {
            let reg = rest.trailing_zeros();
            rest &= rest - 1;
            if l {
                let value = self.load(pc, addr, 4)?;
                if reg == 15 {
                    // Bit 0 of a loaded PC selects the state, which is how Thumb functions
                    // are returned from in interworked builds.
                    self.branch_to(value, true);
                } else {
                    self.r[reg as usize] = value;
                }
            } else {
                let value = if reg == 15 {
                    self.pc + 4
                } else {
                    self.r[reg as usize]
                };
                self.store(pc, addr, 4, value)?;
            }
            addr = addr.wrapping_add(4);
        }
        Ok(())
    }

    /// Group `101`: `B` and `BL`. The whole low 24 bits is the signed word offset, so a `BL`
    /// with bit23 set is a branch past -2MiB, not another instruction. Only a conditional encoding
    /// reaches this arm; BLX-immediate is decoded in [`Cpu::arm_blx_immediate`].
    ///
    /// This comment used to assert that no ARMv5 module built for BREW calls a Thumb function
    /// through BLX-immediate and that it therefore need not be decoded. That is withdrawn: two
    /// reference Resident Evil builds and six QQ builds open with a compiler-generated veneer that
    /// shuffles arguments in ARM and then `blx` into the pack's Thumb body.
    #[inline(always)]
    fn arm_branch(&mut self, insn: u32) -> Result<(), Stop> {
        if insn & bit(24) != 0 {
            self.r[14] = self.pc;
        }
        // The offset is relative to <instruction>+8, but `pc` has already advanced to
        // <instruction>+4, hence the extra 4. LR is <instruction>+4, so returning lands on the
        // instruction after the `BL`.
        self.pc = self.pc.wrapping_add(4).wrapping_add(scaled_offset(insn)) & !3;
        Ok(())
    }

    /// `blx #imm`: bit 24 is the target's halfword bit rather than part of the signed word offset,
    /// and the branch always enters Thumb — which is why a compiler emits it instead of `bl`.
    #[inline(always)]
    fn arm_blx_immediate(&mut self, insn: u32) -> Result<(), Stop> {
        self.r[14] = self.pc;
        let target = self
            .pc
            .wrapping_add(4)
            .wrapping_add(scaled_offset(insn))
            .wrapping_add(((insn >> 24) & 1) << 1);
        // The encoding leaves bit 0 clear, so setting it names the state BLX guarantees rather than
        // asking the address to choose one.
        self.branch_to(target | 1, true);
        Ok(())
    }
}

#[cfg(test)]
#[path = "../test/cpu/arm.rs"]
mod tests;
