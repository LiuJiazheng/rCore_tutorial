use riscv::register::{
    scause::{
        self,
        Trap,
        Exception,
        Interrupt
    },
    sepc,
    stvec,
    sscratch,
    sstatus,
    sie,
};
use crate::context::TrapFrame;
use crate::timer::{
    TICKS,
    clock_set_next_event
};
use crate::process::tick;
use crate::sbi::console_getchar;

global_asm!(include_str!("trap/trap.asm"));

pub fn init() {
    unsafe {
        extern "C" {
            fn __alltraps();
        }
        sscratch::write(0);
        stvec::write(__alltraps as usize, stvec::TrapMode::Direct);
        sstatus::set_sie();

        // enable external interrupt
        sie::set_sext();
    }
    println!("++++ setup interrupt! ++++");
}

#[no_mangle]
pub fn rust_trap(tf: &mut TrapFrame) {
    // println!("rust_trap sepc = {:#x} scause = {:?}", tf.sepc, tf.scause.cause());
    match tf.scause.cause() {
        Trap::Exception(Exception::Breakpoint) => breakpoint(&mut tf.sepc),
        Trap::Exception(Exception::InstructionPageFault) => page_fault(tf),
        Trap::Exception(Exception::LoadPageFault) => page_fault(tf),
        Trap::Exception(Exception::StorePageFault) => page_fault(tf),
        Trap::Exception(Exception::UserEnvCall) => syscall(tf),
        Trap::Interrupt(Interrupt::SupervisorTimer) => super_timer(tf),
        Trap::Interrupt(Interrupt::SupervisorExternal) => external(),
        _ => panic!("trap {:?} not handled!", tf.scause.cause())
    }
}

fn breakpoint(sepc: &mut usize) {
    println!("a breakpoint set @0x{:x}", sepc);
    *sepc += 4;
}

fn page_fault(tf: &mut TrapFrame) {
    println!("{:?} va = {:#x} instruction = {:#x}", tf.scause.cause(), tf.stval, tf.sepc);
    panic!("page fault!");
}

fn super_timer(tf: &mut TrapFrame) {
    //unsafe { sie::clear_stimer(); }
    //println!("in super_timer()!");
    /*
    match tf.sstatus.spp() {
        sstatus::SPP::User => println!("\ntimer from user mode!"),
        sstatus::SPP::Supervisor => println!("\ntimer from supervisor mode!"),
    }
    */
    clock_set_next_event();
    unsafe {
        TICKS += 1;
        if TICKS == 100 {
            TICKS = 0;
            println!("* 100 ticks *");
        }
    }
    //println!("ready tick()...");
    tick();
    //println!("super_timer ends.");
    
    //println!("out super_timer()!");
    //unsafe { sie::set_stimer(); }
}

fn external() {
    //crate::fs::stdio::STDIN.push(console_getchar() as u8 as char);
    //println!("{}", console_getchar() as u8 as char);
    panic!("in external!");
}

#[inline(always)]
pub fn enable_and_wfi() {
    unsafe {
        asm!("csrsi sstatus, 1 << 1; wfi" :::: "volatile");
    }
}

#[inline(always)]
pub fn disable_and_store() -> usize {
    let sstatus: usize;
    unsafe {
        asm!("csrci sstatus, 1 << 1" : "=r"(sstatus) ::: "volatile");
    }
    sstatus & (1 << 1)
}

#[inline(always)]
pub fn restore(flags: usize) {
    unsafe {
        asm!("csrs sstatus, $0" :: "r"(flags) :: "volatile");
    }
}

/*
pub const SYS_WRITE: usize = 64;
pub const SYS_EXIT: usize = 93;

fn syscall(tf: &mut TrapFrame) {
    tf.sepc += 4;
    match tf.x[17] {
        SYS_WRITE => {
            print!("{}", tf.x[10] as u8 as char);
        },
        SYS_EXIT => {
            println!("process exited!");
            crate::process::exit(tf.x[10]);
        },
        _ => {
            panic!("unknown user syscall!");
        }
    }
}
*/

fn syscall(tf: &mut TrapFrame) {
    tf.sepc += 4;
    let ret = crate::syscall::syscall(
        tf.x[17],
        [tf.x[10], tf.x[11], tf.x[12]],
        tf
    );
    tf.x[10] = ret as usize;
}
