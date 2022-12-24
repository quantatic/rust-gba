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

        let start = assembler.offset();
        dynasm!(assembler
            ; .arch x64
            // ; int3
            ; push rbp
            ; mov rbp, rsp
            ; push rdi
            ; sub rsp, 8
        );

        match instruction.instruction_type() {
            ArmInstructionType::B { offset } => {
                dynasm!(assembler
                    ; mov rdi, [rbp - 8]
                    ; mov rsi, Register::R15 as _
                    ; mov rax, QWORD Self::jit_read_register as i64
                    ; call rax
                    ; add eax, offset
                    ; mov rdi, [rbp - 8]
                    ; mov esi, eax
                    ; mov rdx, Register::R15 as _
                    ; mov rax, QWORD Self::jit_write_register as i64
                    ; call rax
                );
            }
            _ => return None,
        }

        dynasm!(assembler
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
        println!("value: {:08X}", value);
        println!("register: {:?}", register);
        self.write_register(value, register);
        println!("done writing register");
    }
}
