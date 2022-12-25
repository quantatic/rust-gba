use dynasmrt::{
    dynasm, x64::X64Relocation, Assembler, AssemblyOffset, DynamicLabel, DynasmApi, DynasmLabelApi,
    ExecutableBuffer,
};

use crate::{
    cpu::{arm::SingleDataTransferOffsetValue, ShiftType},
    Cpu, Register,
};

use super::{
    arm::{
        ArmInstruction, ArmInstructionType, SingleDataMemoryAccessSize,
        SingleDataTransferIndexType, SingleDataTransferOffsetInfo,
    },
    InstructionCondition,
};

pub struct JitInstruction {
    buffer: ExecutableBuffer,
    start: AssemblyOffset,
}

impl JitInstruction {
    pub fn execute(&self, cpu: &mut Cpu) {
        let f: unsafe extern "sysv64" fn(*mut Cpu) =
            unsafe { std::mem::transmute(self.buffer.ptr(self.start)) };
        unsafe { f(cpu as _) }
    }
}

macro_rules! call_self {
    ($ops:ident, $addr:expr) => {
        dynasm!($ops
            ; mov rcx, rdx
            ; mov rdx, rsi
            ; mov rsi, rdi
            ; mov rdi, r12
            ; mov rax, QWORD $addr as _
            ; call rax
        );
    }
}
impl Cpu {
    pub fn try_jit(instruction: ArmInstruction) -> Option<JitInstruction> {
        if !matches!(
            instruction.instruction_type(),
            ArmInstructionType::B { .. } //     | ArmInstructionType::Bl { .. }
                                         //     | ArmInstructionType::Bx { .. }
                                         //     | ArmInstructionType::Ldr { .. }
        ) {
            return None;
        }

        let mut assembler: Assembler<X64Relocation> = Assembler::new().unwrap();

        let start = assembler.offset();
        dynasm!(assembler
            ; .arch x64
            ; push rbp
            ; mov rbp, rsp
            ; push r12
            ; sub rsp, 8

            ; mov r12, rdi // r12 contains self
        );

        let pass_label = assembler.new_dynamic_label();
        let fail_label = assembler.new_dynamic_label();
        Self::emit_conditional_check(
            &mut assembler,
            instruction.instruction_condition(),
            pass_label,
            fail_label,
        );

        dynasm!(assembler
            ; =>pass_label
        );
        match instruction.instruction_type() {
            ArmInstructionType::B { offset } => Self::emit_arm_b(&mut assembler, offset),
            ArmInstructionType::Bl { offset } => Self::emit_arm_bl(&mut assembler, offset),
            ArmInstructionType::Bx { operand } => Self::emit_arm_bx(&mut assembler, operand),
            ArmInstructionType::Ldr {
                index_type,
                base_register,
                destination_register,
                offset_info,
                access_size,
                sign_extend,
            } => Self::emit_arm_ldr(
                &mut assembler,
                access_size,
                base_register,
                destination_register,
                index_type,
                offset_info,
                sign_extend,
            ),
            _ => unreachable!(),
        }

        // if condition fails, ensure we still advance PC
        dynasm!(assembler
            ; jmp ->cleanup
            ; =>fail_label
            ;; call_self!(assembler, Self::jit_advance_pc_for_arm_instruction)
        );

        dynasm!(assembler
            ; ->cleanup:
            ; add rsp, 8
            ; pop r12
            ; mov rsp, rbp
            ; pop rbp
            ; ret
        );

        let buffer = assembler.finalize().unwrap();
        let result = JitInstruction { buffer, start };

        Some(result)
    }

    fn emit_arm_b(assembler: &mut Assembler<X64Relocation>, offset: i32) {
        dynasm!(assembler
            ; mov rdi, Register::R15 as _
            ;; call_self!(assembler, Self::jit_read_register)

            ; add eax, offset

            ; mov edi, eax
            ; mov rsi, Register::R15 as _
            ;; call_self!(assembler, Self::jit_write_register)

            ;; call_self!(assembler, Self::jit_flush_prefetch)
        );
    }

    fn emit_arm_bl(assembler: &mut Assembler<X64Relocation>, offset: i32) {
        dynasm!(assembler
            ; sub rsp, 8
            ; push r13

            ; mov rdi, Register::R15 as _
            ;; call_self!(assembler, Self::jit_read_register)

            ; mov r13d, eax
            ; sub eax, 4

            ; mov edi, eax
            ; mov rsi, Register::R14 as _
            ;; call_self!(assembler, Self::jit_write_register)

            ; add r13d, offset

            ; mov edi, r13d
            ; mov rsi, Register::R15 as _
            ;; call_self!(assembler, Self::jit_write_register)

            ; pop r13
            ; add rsp, 8

            ;; call_self!(assembler, Self::jit_flush_prefetch)
        );
    }

    fn emit_arm_bx(assembler: &mut Assembler<X64Relocation>, operand: Register) {
        dynasm!(assembler
            ; mov rdi, operand as _
            ;; call_self!(assembler, Self::jit_read_register)

            ; mov ecx, eax
            ; and ecx, 1 // state bit
            ; xor eax, ecx // ensure state in address is unset

            ; sub rsp, 8
            ; push rax

            ; mov dil, cl
            ;; call_self!(assembler, Self::jit_set_cpu_state_bit)

            ; pop rax
            ; add rsp, 8

            ; mov edi, eax
            ; mov rsi, Register::R15 as _
            ;; call_self!(assembler, Self::jit_write_register)

            ;; call_self!(assembler, Self::jit_flush_prefetch)
        );
    }

    fn emit_arm_ldr(
        assembler: &mut Assembler<X64Relocation>,
        access_size: SingleDataMemoryAccessSize,
        base_register: Register,
        destination_register: Register,
        index_type: SingleDataTransferIndexType,
        offset_info: SingleDataTransferOffsetInfo,
        sign_extend: bool,
    ) {
        dynasm!(assembler
            ; mov rdi, base_register as _
            ;; call_self!(assembler, Self::jit_read_register)

            ; mov QWORD [rbp - 16], rax // rbp - 16, base_address

            ; sub rsp, 16 // allocate more space on stack
        );

        // rbp - 24, offset_amount
        match offset_info.value {
            SingleDataTransferOffsetValue::Immediate { offset } => dynasm!(assembler
                ; mov DWORD [rbp - 24], offset as _
            ),
            SingleDataTransferOffsetValue::Register { offset_register } => dynasm!(assembler
                ; mov rdi, offset_register as _
                ;; call_self!(assembler, Self::jit_read_register)
                ; mov DWORD [rbp - 24], eax
            ),
            SingleDataTransferOffsetValue::RegisterImmediate {
                shift_amount,
                shift_type,
                offset_register,
            } => {
                dynasm!(assembler
                    ; mov rdi, offset_register as _
                    ;; call_self!(assembler, Self::jit_read_register)
                );

                // offset base in eax
                match shift_type {
                    ShiftType::Lsl => {
                        if shift_amount == 0 {
                            // result already in eax
                        } else {
                            dynasm!(assembler
                                ; shl eax, shift_amount as _
                            );
                        }
                    }
                    ShiftType::Lsr => {
                        if shift_amount == 0 {
                            dynasm!(assembler
                                ; mov eax, 0
                            );
                        } else {
                            dynasm!(assembler
                                ; shr eax, shift_amount as _
                            );
                        }
                    }
                    ShiftType::Asr => {
                        if shift_amount == 0 {
                            dynasm!(assembler
                                ; sar eax, 31
                            );
                        } else {
                            dynasm!(assembler
                                ; sar eax, shift_amount as _
                            );
                        }
                    }
                    ShiftType::Ror => {
                        if shift_amount == 0 {
                            dynasm!(assembler
                                ; shr eax, 1
                                ; mov DWORD [rbp - 32], eax // save in >> 1

                                ;; call_self!(assembler, Self::jit_get_carry_flag)

                                ; shl eax, 31
                                ; or eax, DWORD [rbp - 32]
                            );
                        } else {
                            dynasm!(assembler
                                ; ror eax, shift_amount as _
                            );
                        }
                    }
                }

                dynasm!(assembler
                    ; mov DWORD [rbp - 24], eax
                );
            }
        }

        // [rbp - 32], offset address
        if offset_info.sign {
            dynasm!(assembler
                ; mov eax, [rbp - 16]
                ; sub eax, [rbp - 24]
                ; mov [rbp - 32], eax
            );
        } else {
            dynasm!(assembler
                ; mov eax, [rbp - 16]
                ; add eax, [rbp - 24]
                ; mov [rbp - 32], eax
            );
        }

        dynasm!(assembler
            ; sub rsp, 16
        );

        // eax, data_read_address
        match index_type {
            SingleDataTransferIndexType::PostIndex { .. } => {
                // post index always has write-back
                dynasm!(assembler
                    ; mov edi, [rbp - 32]
                    ; mov rsi, base_register as _
                    ;; call_self!(assembler, Self::jit_write_register)

                    ; mov eax, [rbp - 16]
                );
            }
            SingleDataTransferIndexType::PreIndex { write_back } => {
                if write_back {
                    dynasm!(assembler
                        ; mov edi, [rbp - 32]
                        ; mov rsi, base_register as _
                        ;; call_self!(assembler, Self::jit_write_register)
                    );
                }

                dynasm!(assembler
                    ; mov eax, [rbp - 32]
                );
            }
        }

        // eax, value
        match (access_size, sign_extend) {
            (SingleDataMemoryAccessSize::Byte, false) => {
                dynasm!(assembler
                    ; mov edi, eax
                    ;; call_self!(assembler, Self::jit_read_byte_address)
                );
            }
            (SingleDataMemoryAccessSize::Byte, true) => {
                dynasm!(assembler
                    ; mov edi, eax
                    ;; call_self!(assembler, Self::jit_read_byte_address)
                    ; movsx eax, al
                );
            }
            (SingleDataMemoryAccessSize::HalfWord, false) => {
                dynasm!(assembler
                    ; mov ecx, eax
                    ; and ecx, 1
                    ; shl cl, 3 // * 8
                    ; mov [rbp - 40], cl // save cl through function call

                    ; mov edi, eax
                    ;; call_self!(assembler, Self::jit_read_halfword_address)

                    ; mov cl, [rbp - 40]
                    ; ror eax, cl
                );
            }
            (SingleDataMemoryAccessSize::HalfWord, true) => {
                // LDRSH Rd,[odd]  -->  LDRSB Rd,[odd]         ;sign-expand BYTE value
                dynasm!(assembler
                    ; bt eax, 1 // if carry flag set, unaligned
                    ; jc >unaligned

                    ; aligned:
                    ; mov edi, eax
                    ;; call_self!(assembler, Self::jit_read_halfword_address)
                    ; movsx eax, ax
                    ; jmp >after

                    ; unaligned:
                    ; mov edi, eax
                    ;; call_self!(assembler, Self::jit_read_byte_address)
                    ; movsx eax, ax

                    ; after:
                );
            }
            (SingleDataMemoryAccessSize::Word, false) => {
                dynasm!(assembler
                    ; mov ecx, eax
                    ; and ecx, 0b11 // & 0b11
                    ; shl cl, 3 // * 8
                    ; mov [rbp - 40], cl


                    ; mov edi, eax
                    ;; call_self!(assembler, Self::jit_read_word_address)

                    ; mov cl, [rbp - 40]
                    ; ror eax, cl
                );
            }
            (SingleDataMemoryAccessSize::Word, true) => unreachable!(),
            _ => todo!("{:?} sign extend: {}", access_size, sign_extend),
        };

        dynasm!(assembler
            ; mov edi, eax
            ; mov rsi, destination_register as _
            ;; call_self!(assembler, Self::jit_write_register)
        );

        if matches!(destination_register, Register::R15) {
            dynasm!(assembler
                ;; call_self!(assembler, Self::jit_flush_prefetch)
            );
        } else {
            dynasm!(assembler
                ;; call_self!(assembler, Self::advance_pc_for_arm_instruction)
            );
        }
    }

    fn emit_conditional_check(
        assembler: &mut Assembler<X64Relocation>,
        condition: InstructionCondition,
        pass_label: DynamicLabel,
        fail_label: DynamicLabel,
    ) {
        match condition {
            InstructionCondition::Always => dynasm!(assembler
                ; jmp =>pass_label
            ),
            InstructionCondition::Never => dynasm!(assembler
                ; jmp =>fail_label
            ),
            InstructionCondition::Equal => dynasm!(assembler
                ;; call_self!(assembler, Self::jit_get_zero_flag)
                ; cmp al, true as _
                ; je =>pass_label
                ; jmp =>fail_label
            ),
            InstructionCondition::NotEqual => dynasm!(assembler
                ;; call_self!(assembler, Self::jit_get_zero_flag)
                ; cmp al, false as _
                ; je =>pass_label
                ; jmp => fail_label
            ),
            InstructionCondition::UnsignedHigherOrSame => dynasm!(assembler
                ;; call_self!(assembler, Self::jit_get_carry_flag)
                ; cmp al, true as _
                ; je =>pass_label
                ; jmp =>fail_label
            ),
            InstructionCondition::UnsignedLower => dynasm!(assembler
                ;; call_self!(assembler, Self::jit_get_carry_flag)
                ; cmp al, false as _
                ; je =>pass_label
                ; jmp =>fail_label
            ),
            InstructionCondition::SignedNegative => dynasm!(assembler
                ;; call_self!(assembler, Self::jit_get_sign_flag)
                ; cmp al, true as _
                ; je =>pass_label
                ; jmp =>fail_label
            ),
            InstructionCondition::SignedPositiveOrZero => dynasm!(assembler
                ;; call_self!(assembler, Self::jit_get_sign_flag)
                ; cmp al, false as _
                ; je =>pass_label
                ; jmp =>fail_label
            ),
            InstructionCondition::SignedOverflow => dynasm!(assembler
                ;; call_self!(assembler, Self::jit_get_overflow_flag)
                ; cmp al, true as _
                ; je =>pass_label
                ; jmp =>fail_label
            ),
            InstructionCondition::SignedNoOverflow => dynasm!(assembler
                ;; call_self!(assembler, Self::jit_get_overflow_flag)
                ; cmp al, false as _
                ; je =>pass_label
                ; jmp =>fail_label
            ),
            InstructionCondition::UnsignedHigher => dynasm!(assembler
                ;; call_self!(assembler, Self::jit_get_carry_flag)
                ; cmp al, true as _
                ; jne =>fail_label
                ;; call_self!(assembler, Self::jit_get_zero_flag)
                ; cmp al, true as _
                ; je =>fail_label
                ; jmp =>pass_label
            ),
            InstructionCondition::UnsignedLowerOrSame => dynasm!(assembler
                ;; call_self!(assembler, Self::jit_get_carry_flag)
                ; cmp al, false as _
                ; je =>pass_label
                ;; call_self!(assembler, Self::jit_get_zero_flag)
                ; cmp al, true as _
                ; je =>pass_label
                ; jmp =>fail_label
            ),
            InstructionCondition::SignedGreaterOrEqual => dynasm!(assembler
                ;; call_self!(assembler, Self::jit_get_sign_flag)
                ; push rax
                ; push rax
                ;; call_self!(assembler, Self::jit_get_overflow_flag)
                ; cmp [rsp], al
                ; pop rax
                ; pop rax
                ; je =>pass_label
                ; jmp =>fail_label
            ),
            InstructionCondition::SignedLessThan => dynasm!(assembler
                ;; call_self!(assembler, Self::jit_get_sign_flag)
                ; push rax
                ; push rax
                ;; call_self!(assembler, Self::jit_get_overflow_flag)
                ; cmp [rsp], al
                ; pop rax
                ; pop rax
                ; jne =>pass_label
                ; jmp =>fail_label
            ),
            InstructionCondition::SignedGreaterThan => dynasm!(assembler
                ;; call_self!(assembler, Self::jit_get_zero_flag)
                ; cmp al, false as _
                ; jne =>fail_label
                ;; call_self!(assembler, Self::jit_get_sign_flag)
                ; push rax
                ; push rax
                ;; call_self!(assembler, Self::jit_get_overflow_flag)
                ; cmp [rsp], al
                ; pop rax
                ; pop rax
                ; je =>pass_label
                ; jmp =>fail_label
            ),
            InstructionCondition::SignedLessOrEqual => dynasm!(assembler
                ;; call_self!(assembler, Self::jit_get_zero_flag)
                ; cmp al, true as _
                ; je =>pass_label
                ;; call_self!(assembler, Self::jit_get_sign_flag)
                ; push rax
                ; push rax
                ;; call_self!(assembler, Self::jit_get_overflow_flag)
                ; cmp [rsp], al
                ; pop rax
                ; pop rax
                ; jne =>pass_label
                ; jmp =>fail_label
            ),
        }
    }

    extern "sysv64" fn jit_read_register(&self, register: Register) -> u32 {
        let result = self.read_register(register, |pc| pc);
        result
    }

    extern "sysv64" fn jit_write_register(&mut self, value: u32, register: Register) {
        self.write_register(value, register);
    }

    extern "sysv64" fn jit_flush_prefetch(&mut self) {
        self.flush_prefetch();
    }

    extern "sysv64" fn jit_advance_pc_for_arm_instruction(&mut self) {
        self.advance_pc_for_arm_instruction();
    }

    extern "sysv64" fn jit_get_zero_flag(&self) -> bool {
        self.get_zero_flag()
    }

    extern "sysv64" fn jit_get_carry_flag(&self) -> bool {
        self.get_carry_flag()
    }

    extern "sysv64" fn jit_get_sign_flag(&self) -> bool {
        self.get_sign_flag()
    }

    extern "sysv64" fn jit_get_overflow_flag(&self) -> bool {
        self.get_overflow_flag()
    }

    extern "sysv64" fn jit_set_cpu_state_bit(&mut self, set: bool) {
        self.set_cpu_state_bit(set)
    }

    extern "sysv64" fn jit_read_byte_address(&mut self, address: u32) -> u8 {
        self.bus.read_byte_address(address)
    }

    extern "sysv64" fn jit_read_halfword_address(&mut self, address: u32) -> u16 {
        self.bus.read_halfword_address(address)
    }

    extern "sysv64" fn jit_read_word_address(&mut self, address: u32) -> u32 {
        self.bus.read_word_address(address)
    }
}
