#![feature(lang_items)]
#![feature(start)]
#![feature(llvm_asm)]
#![feature(asm)]
#![feature(alloc_error_handler)]
#![no_std]
#![allow(dead_code)]

mod aarch64;
mod bits;
mod boot;
mod driver;
mod el0;
mod el1;
mod el2;
mod el3;
mod pager;
mod psci;
mod slab;

#[macro_use]
extern crate alloc;

use core::panic::PanicInfo;

pub fn print_msg(key: &str, val: &str) {
    driver::uart::puts("[");
    driver::uart::puts(key);
    for _ in key.len()..12 {
        driver::uart::puts(" ");
    }
    driver::uart::puts("] ");
    driver::uart::puts(val);
    driver::uart::puts("\n");
}

#[no_mangle]
pub fn ns_entry() -> ! {
    driver::delays::forever();
}

/// initialization for the master CPU
fn init_master() {
    if aarch64::cpu::get_current_el() == 3 {
        aarch64::cpu::init_cptr_el3();
    }

    driver::early_init();

    match aarch64::mmu::init() {
        Some(_) => (),
        None => {
            panic!("failed to initialize MMU");
        }
    };
    driver::init();

    // examples
    // driver::psci::pwr_domain_on(1); // wake up CPU #1 (Pine64)
    // aarch64::cpu::start_non_primary(); // wake up non-primary CPUs (Raspi)

    match aarch64::cpu::get_current_el() {
        3 => {
            psci::init();
            aarch64::context::init_secure();
            print_msg("PSCI", "enabled");
            boot::run();
            aarch64::context::init_el2_regs();
            el3::el3_to_el1();
        }
        2 => {
            print_msg("Warning", "execution level is not EL3");
            print_msg("PSCI", "disabled");
            boot::run();
            aarch64::context::init_el2_regs();
            el2::el2_to_el1();
        }
        _ => {
            panic!("execution level is not EL3");
        }
    }
}

/// initialization for slave CPUs
fn init_slave() -> ! {
    aarch64::mmu::set_regs();
    driver::uart::puts("initialized slaves\n");
    driver::delays::forever()
}

#[no_mangle]
pub fn entry() -> ! {
    aarch64::mmu::init_memory_map();

    if driver::topology::core_pos() == 0 {
        init_master();
    } else {
        init_slave();
    }

    driver::delays::forever()
}

#[lang = "eh_personality"]
#[no_mangle]
extern "C" fn eh_personality() {}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    driver::uart::puts("kernel panic!\n");
    if let Some(location) = info.location() {
        driver::uart::puts(location.file());
        driver::uart::puts(":");
        driver::uart::decimal(location.line() as u64);
        driver::uart::puts("\n");
    }

    if let Some(s) = info.payload().downcast_ref::<&str>() {
        driver::uart::puts(s);
        driver::uart::puts("\n");
    }

    driver::delays::forever();
}

#[no_mangle]
pub fn abort() -> ! {
    driver::delays::forever();
}
