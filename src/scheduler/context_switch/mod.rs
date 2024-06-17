pub struct Task<'a> {
    pub task_handler: *const TaskHandler,
    pub stack_pointer: *mut u32,
    pub stack_size: u32,
    pub stack: &'a mut [u32]
}
pub unsafe trait TaskConcurrent {
    unsafe fn init<'a>(task_handler: *const TaskHandler, stack_stop: *const (), stack_size: u32) -> Task<'a>;
}

pub type TaskHandler = fn() -> ();

pub unsafe fn new_task<'a>(task_handler: *const TaskHandler, stack: &'a mut [u32]) -> Task<'a> {
    let stack_stop: *const () = stack.as_ptr() as *const ();
    let stack_size: u32 = stack.len() as u32;
    
    // abrunden des Top of Stack auf 8 Byte
    // MERKE: ARM Cortex-M stack waechst nach unten:
    // hohe -> niedrige Speicheradressen
    let mut sp: *mut u32 = (((stack_stop as u32 + stack_size) / 8) * 8) as *mut u32;
    
    sp = sp.byte_offset(-1); *sp = 1 << 24;  // xPSR Bit 24 high
    sp = sp.byte_offset(-1); *sp = task_handler as u32;
    // Befuellen der Speicherbereiche mit Dummywerten
    sp = sp.byte_offset(-1); *sp = 0x0000000E;  // LR - Link Register
    sp = sp.byte_offset(-1); *sp = 0x0000000C;  // R12
    sp = sp.byte_offset(-1); *sp = 0x00000003;  // R3
    sp = sp.byte_offset(-1); *sp = 0x00000002;  // R2
    sp = sp.byte_offset(-1); *sp = 0x00000001;  // R1
    sp = sp.byte_offset(-1); *sp = 0x00000000;  // R0
    // Speicherbereiche für Register R4 bis R11
    sp = sp.byte_offset(-1); *sp = 0x0000000B;  // R11
    sp = sp.byte_offset(-1); *sp = 0x0000000A;  // R10
    sp = sp.byte_offset(-1); *sp = 0x00000009;  // R9
    sp = sp.byte_offset(-1); *sp = 0x00000008;  // R8
    sp = sp.byte_offset(-1); *sp = 0x00000007;  // R7
    sp = sp.byte_offset(-1); *sp = 0x00000006;  // R6
    sp = sp.byte_offset(-1); *sp = 0x00000005;  // R5
    sp = sp.byte_offset(-1); *sp = 0x00000004;  // R4

    let ret_val = Task {
        task_handler: task_handler,
        stack_pointer: sp,  // Stack Pointer speichern
        stack_size: stack_size,
        stack: stack
    };

    // aufrunden des Bottom of Stack auf 8 Byte
    let stack_limit: *mut u32 = ((((stack_stop as u32 - 1) / 8) + 1) * 8) as *mut u32;

    // Fuellen des unbenutzten Stacks mit 0xDEADBEEF
    sp = sp.byte_offset(-1);
    loop {
        if sp >= stack_limit { break; }
        *sp = 0xDEADBEEF;
        sp = sp.byte_offset(-1);
    }

    return ret_val;
}