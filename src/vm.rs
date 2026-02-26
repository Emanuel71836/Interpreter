use crate::value::Value;
use crate::bytecode::{Instruction, OpCode};
use crate::arena::Arena;
use std::rc::Rc;

struct Frame {
    registers: Vec<Value>,
    program: Rc<Vec<Instruction>>,
    pc: usize,
    return_register: u8,
    saved_regs: Vec<Value>, // simple stack to save registers across calls
}

impl Frame {
    fn new(program: Rc<Vec<Instruction>>, num_regs: usize) -> Self {
        Frame {
            registers: vec![Value::nil(); num_regs],
            program,
            pc: 0,
            return_register: 0,
            saved_regs: Vec::new(),
        }
    }
}

pub struct VM {
    frames: Vec<Frame>,
    functions: Vec<(Rc<Vec<Instruction>>, usize, usize)>, // bytecode, #params, #regs
    string_pool: Vec<String>,                              // original string data
    arena: Arena,
}

impl VM {
    pub fn new(
        functions: Vec<(Rc<Vec<Instruction>>, usize, usize)>,
        string_pool: Vec<String>,
        arena_capacity: usize,
    ) -> Self {
        let main_frame = Frame::new(functions[0].0.clone(), functions[0].2);
        VM {
            frames: vec![main_frame],
            functions,
            string_pool,
            arena: Arena::new(arena_capacity),
        }
    }

    pub fn run(&mut self) -> Result<(), String> {
        while !self.frames.is_empty() {
            let frame = self.frames.last_mut().unwrap();
            if frame.pc >= frame.program.len() {
                return Err("Program ended without return".to_string());
            }
            let insn = frame.program[frame.pc];
            frame.pc += 1;

            match insn.opcode() {
                OpCode::Add => {
                    let dst = insn.dst() as usize;
                    let a = frame.registers[insn.src1() as usize];
                    let b = frame.registers[insn.src2() as usize];
                    frame.registers[dst] = a + b;
                }
                OpCode::Sub => {
                    let dst = insn.dst() as usize;
                    let a = frame.registers[insn.src1() as usize];
                    let b = frame.registers[insn.src2() as usize];
                    frame.registers[dst] = a - b;
                }
                OpCode::Mul => {
                    let dst = insn.dst() as usize;
                    let a = frame.registers[insn.src1() as usize];
                    let b = frame.registers[insn.src2() as usize];
                    frame.registers[dst] = a * b;
                }
                OpCode::Div => {
                    let dst = insn.dst() as usize;
                    let a = frame.registers[insn.src1() as usize];
                    let b = frame.registers[insn.src2() as usize];
                    frame.registers[dst] = a / b;
                }
                OpCode::Lt => {
                    let dst = insn.dst() as usize;
                    let a = frame.registers[insn.src1() as usize];
                    let b = frame.registers[insn.src2() as usize];
                    frame.registers[dst] = a.lt(b);
                }
                OpCode::LoadConst => {
                    let dst = insn.dst() as usize;
                    let imm = insn.imm() as i64;
                    frame.registers[dst] = Value::from_int(imm);
                }
                OpCode::LoadBool => {
                    let dst = insn.dst() as usize;
                    let b = insn.imm() != 0;
                    frame.registers[dst] = Value::from_bool(b);
                }
                OpCode::LoadNil => {
                    let dst = insn.dst() as usize;
                    frame.registers[dst] = Value::nil();
                }
                OpCode::LoadString => {
                    let dst = insn.dst() as usize;
                    let idx = insn.imm() as usize;
                    if idx >= self.string_pool.len() {
                        panic!("LoadString: index {} out of bounds", idx);
                    }
                    let s = &self.string_pool[idx];
                    frame.registers[dst] = Value::from_string_in_arena(s, &mut self.arena);
                }
                OpCode::Jump => {
                    let target = insn.imm() as usize;
                    frame.pc = target;
                }
                OpCode::Branch => {
                    let cond_reg = insn.dst() as usize;
                    let target = insn.imm() as usize;
                    let cond = frame.registers[cond_reg];
                    let take = cond.to_bool().unwrap_or(true);
                    if take {
                        frame.pc = target;
                    }
                }
                OpCode::Return => {
                    let val_reg = insn.dst() as usize;
                    let ret_val = frame.registers[val_reg];
                    self.frames.pop();
                    if let Some(caller) = self.frames.last_mut() {
                        let ret_dst = caller.return_register as usize;
                        caller.registers[ret_dst] = ret_val;
                        // restore saved registers
                        if !caller.saved_regs.is_empty() {
                            for (i, &val) in caller.saved_regs.iter().enumerate() {
                                if i < caller.registers.len() {
                                    caller.registers[i] = val;
                                }
                            }
                            caller.saved_regs.clear();
                        }
                    } else {
                        println!("Returned: {:?}", ret_val);
                        return Ok(());
                    }
                }
                OpCode::Print => {
                    let reg = insn.dst() as usize;
                    let val = frame.registers[reg];
                    println!("{:?}", val);
                }
                OpCode::Move => {
                    let dst = insn.dst() as usize;
                    let src = insn.src1() as usize;
                    frame.registers[dst] = frame.registers[src];
                }
                OpCode::Call => {
                    let func_idx = insn.imm() as usize;
                    if func_idx >= self.functions.len() {
                        panic!("Call to undefined function index {}", func_idx);
                    }
                    let (callee_bytecode, num_params, max_reg) = self.functions[func_idx].clone();

                    // save all registers (simplistic)
                    frame.saved_regs = frame.registers.clone();

                    let mut new_frame = Frame::new(callee_bytecode, max_reg);
                    new_frame.return_register = 0;
                    // copy arguments into new frame's registers 0..num_params-1
                    for i in 0..num_params {
                        new_frame.registers[i] = frame.registers[i];
                    }
                    self.frames.push(new_frame);
                }
            }
        }
        Ok(())
    }
}