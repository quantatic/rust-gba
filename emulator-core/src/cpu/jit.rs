use dynasmrt::{
    dynasm, x64::X64Relocation, Assembler, AssemblyOffset, DynasmApi, DynasmLabelApi,
    ExecutableBuffer,
};

use crate::{Cpu, Register};

use super::{
    arm::{ArmInstruction, ArmInstructionType},
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

impl Cpu {
    pub fn try_jit(instruction: ArmInstruction) -> Option<JitInstruction> {
        let mut assembler: Assembler<X64Relocation> = Assembler::new().unwrap();

        let start = assembler.offset();
        return None;
        dynasm!(assembler
            ; .arch x64
            ; push rbp
            ; mov rbp, rsp
            ; push rdi
            ; sub rsp, 8
        );

        dynasm!(assembler
            ; mov rdi, [rbp - 8]
            ; mov rsi, instruction.instruction_condition() as _
            ; mov rax, QWORD Self::jit_evaluate_instruction_condition as _
            ; call rax
            ; cmp al, 0
            ; je ->condition_failed
        );

        match instruction.instruction_type() {
            ArmInstructionType::B { offset } => {
                dynasm!(assembler
                    ; mov rdi, [rbp - 8]
                    ; mov rsi, Register::R15 as _
                    ; mov rax, QWORD Self::jit_read_register as _
                    ; call rax

                    ; add eax, offset

                    ; mov rdi, [rbp - 8]
                    ; mov esi, eax
                    ; mov rdx, Register::R15 as _
                    ; mov rax, QWORD Self::jit_write_register as _
                    ; call rax

                    ; mov rdi, [rbp - 8]
                    ; mov rax, QWORD Self::jit_flush_prefetch as _
                    ; call rax
                    ; jmp ->cleanup
                );
            }
            _ => return None,
        }

        // if condition fails, ensure we still advance PC
        dynasm!(assembler
            ; ->condition_failed:
            ; mov rdi, [rbp - 8]
            ; mov rax, QWORD Self::jit_advance_pc_for_arm_instruction as i64
            ; call rax
        );

        dynasm!(assembler
            ; ->cleanup:
            ; mov rsp, rbp
            ; pop rbp
            ; ret
        );

        let buffer = assembler.finalize().unwrap();
        let result = JitInstruction { buffer, start };

        Some(result)
    }

    extern "sysv64" fn jit_read_register(&self, register: Register) -> u32 {
        self.read_register(register, |pc| pc)
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

    extern "sysv64" fn jit_evaluate_instruction_condition(
        &self,
        condition: InstructionCondition,
    ) -> bool {
        self.evaluate_instruction_condition(condition)
    }
}
