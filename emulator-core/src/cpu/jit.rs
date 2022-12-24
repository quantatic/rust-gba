use dynasmrt::{
    dynasm, x64::X64Relocation, Assembler, AssemblyOffset, DynasmApi, ExecutableBuffer,
};

use crate::{Cpu, Register};

use super::arm::{ArmInstruction, ArmInstructionType};

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

        dynasm!(assembler
            ; .arch x64
            ; int3
            ; push rbp
            ; mov rbp, rsp
            ; push rdi
        );
        let start = assembler.offset();

        match instruction.instruction_type() {
            ArmInstructionType::B { offset } => {
                dynasm!(assembler
                    ; mov rdi, [rsp]
                    ; mov sil, Register::R15 as _
                    ; call Self::jit_read_register as _
                    ; add eax, offset
                    ; mov rdi, [rsp]
                    ; mov esi, eax
                    ; mov dl, Register::R15 as _
                    ; call Self::jit_write_register as _
                    ; ret
                );
            }
            _ => return None,
        }

        dynasm!(assembler
            ; mov rsp, rbp
            ; pop rbp
        );

        let buffer = assembler.finalize().unwrap();
        let result = JitInstruction { buffer, start };

        Some(result)
    }

    extern "sysv64" fn jit_read_register(&self, register: Register) -> u32 {
        self.read_register(register, |_| unreachable!())
    }

    extern "sysv64" fn jit_write_register(&mut self, value: u32, register: Register) {
        self.write_register(value, register);
    }
}
